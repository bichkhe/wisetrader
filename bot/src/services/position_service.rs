//! Position and Trade Management Service

use anyhow::Result;
use std::sync::Arc;
use sea_orm::{EntityTrait, ActiveValue, ColumnTrait, QueryFilter, QueryOrder, Order};
use shared::entity::{positions, trades, live_trading_signals};
use chrono::Utc;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Create a new position when buy signal is executed
pub async fn create_position(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    order_id: Option<u64>,
    strategy_id: Option<u64>,
    strategy_name: Option<String>,
    exchange: String,
    pair: String,
    entry_price: f64,
    quantity: f64,
) -> Result<u64, anyhow::Error> {
    // Convert f64 to Decimal via string
    let entry_price_decimal = Decimal::from_str(&entry_price.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    let quantity_decimal = Decimal::from_str(&quantity.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    let entry_value_decimal = entry_price_decimal * quantity_decimal;
    
    let position = positions::ActiveModel {
        user_id: ActiveValue::Set(user_id),
        order_id: ActiveValue::Set(order_id),
        strategy_id: ActiveValue::Set(strategy_id),
        strategy_name: ActiveValue::Set(strategy_name),
        exchange: ActiveValue::Set(exchange),
        pair: ActiveValue::Set(pair),
        side: ActiveValue::Set("buy".to_string()),
        entry_price: ActiveValue::Set(entry_price_decimal),
        quantity: ActiveValue::Set(quantity_decimal),
        entry_value: ActiveValue::Set(entry_value_decimal),
        current_price: ActiveValue::Set(Some(entry_price_decimal)),
        unrealized_pnl: ActiveValue::Set(Decimal::ZERO),
        unrealized_pnl_percent: ActiveValue::Set(Decimal::ZERO),
        status: ActiveValue::Set("open".to_string()),
        entry_time: ActiveValue::Set(Some(Utc::now())),
        close_time: ActiveValue::NotSet,
        created_at: ActiveValue::Set(Some(Utc::now())),
        updated_at: ActiveValue::Set(Some(Utc::now())),
        ..Default::default()
    };
    
    let result = positions::Entity::insert(position)
        .exec(db)
        .await?;
    
    Ok(result.last_insert_id)
}

/// Close a position and create a trade record when sell signal is executed
pub async fn close_position_and_create_trade(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    position_id: u64,
    sell_order_id: Option<u64>,
    exit_price: f64,
) -> Result<u64, anyhow::Error> {
    // Get the position
    let position = positions::Entity::find_by_id(position_id)
        .filter(positions::Column::UserId.eq(user_id))
        .filter(positions::Column::Status.eq("open"))
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Position not found or already closed"))?;
    
    // Convert Decimal to f64 for calculations
    let entry_price: f64 = f64::from_str(&position.entry_price.to_string()).unwrap_or(0.0);
    let quantity: f64 = f64::from_str(&position.quantity.to_string()).unwrap_or(0.0);
    let entry_value: f64 = f64::from_str(&position.entry_value.to_string()).unwrap_or(0.0);
    
    let exit_value = exit_price * quantity;
    let pnl = exit_value - entry_value;
    let pnl_percent = if entry_value > 0.0 {
        (pnl / entry_value) * 100.0
    } else {
        0.0
    };
    
    let entry_time = position.entry_time.unwrap_or(Utc::now());
    let exit_time = Utc::now();
    let duration = (exit_time - entry_time).num_seconds();
    
    // Clone values before moving
    let position_exchange = position.exchange.clone();
    let position_pair = position.pair.clone();
    let position_strategy_id = position.strategy_id;
    let position_strategy_name = position.strategy_name.clone();
    let position_order_id = position.order_id;
    
    // Create trade record
    let trade = trades::ActiveModel {
        user_id: ActiveValue::Set(user_id),
        position_id: ActiveValue::Set(Some(position_id)),
        buy_order_id: ActiveValue::Set(position_order_id),
        sell_order_id: ActiveValue::Set(sell_order_id),
        strategy_id: ActiveValue::Set(position_strategy_id),
        strategy_name: ActiveValue::Set(position_strategy_name),
        exchange: ActiveValue::Set(position_exchange),
        pair: ActiveValue::Set(position_pair),
        entry_price: ActiveValue::Set(entry_price.to_string()),
        exit_price: ActiveValue::Set(exit_price.to_string()),
        quantity: ActiveValue::Set(quantity.to_string()),
        entry_value: ActiveValue::Set(entry_value.to_string()),
        exit_value: ActiveValue::Set(exit_value.to_string()),
        pnl: ActiveValue::Set(pnl.to_string()),
        pnl_percent: ActiveValue::Set(pnl_percent.to_string()),
        entry_time: ActiveValue::Set(Some(entry_time)),
        exit_time: ActiveValue::Set(Some(exit_time)),
        duration: ActiveValue::Set(Some(duration)),
        created_at: ActiveValue::Set(Some(Utc::now())),
        ..Default::default()
    };
    
    let trade_result = trades::Entity::insert(trade)
        .exec(db)
        .await?;
    
    // Convert f64 to Decimal for database
    let exit_price_decimal = Decimal::from_str(&exit_price.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    let pnl_decimal = Decimal::from_str(&pnl.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    let pnl_percent_decimal = Decimal::from_str(&pnl_percent.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    
    // Update position status to closed
    let mut position_update: positions::ActiveModel = position.into();
    position_update.status = ActiveValue::Set("closed".to_string());
    position_update.close_time = ActiveValue::Set(Some(exit_time));
    position_update.current_price = ActiveValue::Set(Some(exit_price_decimal));
    position_update.unrealized_pnl = ActiveValue::Set(pnl_decimal);
    position_update.unrealized_pnl_percent = ActiveValue::Set(pnl_percent_decimal);
    position_update.updated_at = ActiveValue::Set(Some(Utc::now()));
    
    positions::Entity::update(position_update)
        .exec(db)
        .await?;
    
    Ok(trade_result.last_insert_id)
}

/// Update position current price and unrealized P&L
pub async fn update_position_price(
    db: &sea_orm::DatabaseConnection,
    position_id: u64,
    current_price: f64,
) -> Result<(), anyhow::Error> {
    let position = positions::Entity::find_by_id(position_id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Position not found"))?;
    
    if position.status != "open" {
        return Ok(()); // Don't update closed positions
    }
    
    // Convert Decimal to f64 for calculations
    let entry_price: f64 = f64::from_str(&position.entry_price.to_string()).unwrap_or(0.0);
    let quantity: f64 = f64::from_str(&position.quantity.to_string()).unwrap_or(0.0);
    let entry_value: f64 = f64::from_str(&position.entry_value.to_string()).unwrap_or(0.0);
    
    // Calculate unrealized P&L
    let current_value = current_price * quantity;
    let unrealized_pnl = current_value - entry_value;
    let unrealized_pnl_percent = if entry_value > 0.0 {
        (unrealized_pnl / entry_value) * 100.0
    } else {
        0.0
    };
    
    // Convert f64 to Decimal for database
    let current_price_decimal = Decimal::from_str(&current_price.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    let unrealized_pnl_decimal = Decimal::from_str(&unrealized_pnl.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    let unrealized_pnl_percent_decimal = Decimal::from_str(&unrealized_pnl_percent.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);
    
    let mut position_update: positions::ActiveModel = position.into();
    position_update.current_price = ActiveValue::Set(Some(current_price_decimal));
    position_update.unrealized_pnl = ActiveValue::Set(unrealized_pnl_decimal);
    position_update.unrealized_pnl_percent = ActiveValue::Set(unrealized_pnl_percent_decimal);
    position_update.updated_at = ActiveValue::Set(Some(Utc::now()));
    
    positions::Entity::update(position_update)
        .exec(db)
        .await?;
    
    Ok(())
}

/// Get all open positions for a user
pub async fn get_open_positions(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
) -> Result<Vec<positions::Model>, anyhow::Error> {
    let positions_list = positions::Entity::find()
        .filter(positions::Column::UserId.eq(user_id))
        .filter(positions::Column::Status.eq("open"))
        .order_by(positions::Column::EntryTime, Order::Desc)
        .all(db)
        .await?;
    
    Ok(positions_list)
}

/// Check if user has an open position for a specific trading pair
pub async fn has_open_position_for_pair(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    pair: &str,
) -> Result<bool, anyhow::Error> {
    let open_positions = get_open_positions(db, user_id).await?;
    Ok(open_positions.iter().any(|p| p.pair == pair && p.status == "open"))
}

/// Get all closed trades for a user
pub async fn get_user_trades(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    limit: Option<u64>,
) -> Result<Vec<trades::Model>, anyhow::Error> {
    let mut trades_list = trades::Entity::find()
        .filter(trades::Column::UserId.eq(user_id))
        .order_by(trades::Column::ExitTime, Order::Desc)
        .all(db)
        .await?;
    
    // Apply limit manually if specified
    if let Some(limit_val) = limit {
        trades_list.truncate(limit_val as usize);
    }
    
    Ok(trades_list)
}

/// Calculate P&L summary for a user
pub async fn calculate_pnl_summary(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
) -> Result<PnlSummary, anyhow::Error> {
    // Get open positions
    let open_positions = get_open_positions(db, user_id).await?;
    
    // Calculate unrealized P&L from open positions
    let mut total_unrealized_pnl = 0.0;
    let mut total_unrealized_pnl_percent = 0.0;
    let mut total_position_value = 0.0;
    
    for position in &open_positions {
        let unrealized_pnl: f64 = f64::from_str(&position.unrealized_pnl.to_string()).unwrap_or(0.0);
        let entry_value: f64 = f64::from_str(&position.entry_value.to_string()).unwrap_or(0.0);
        total_unrealized_pnl += unrealized_pnl;
        total_position_value += entry_value;
    }
    
    if total_position_value > 0.0 {
        total_unrealized_pnl_percent = (total_unrealized_pnl / total_position_value) * 100.0;
    }
    
    // Get closed trades
    let closed_trades = get_user_trades(db, user_id, None).await?;
    
    // Calculate realized P&L from closed trades
    let mut total_realized_pnl = 0.0;
    let mut total_realized_pnl_percent = 0.0;
    let mut total_entry_value = 0.0;
    let mut winning_trades = 0;
    let mut losing_trades = 0;
    
    for trade in &closed_trades {
        let pnl: f64 = trade.pnl.parse().unwrap_or(0.0);
        let entry_value: f64 = trade.entry_value.parse().unwrap_or(0.0);
        total_realized_pnl += pnl;
        total_entry_value += entry_value;
        
        if pnl > 0.0 {
            winning_trades += 1;
        } else if pnl < 0.0 {
            losing_trades += 1;
        }
    }
    
    if total_entry_value > 0.0 {
        total_realized_pnl_percent = (total_realized_pnl / total_entry_value) * 100.0;
    }
    
    let total_pnl = total_unrealized_pnl + total_realized_pnl;
    let win_rate = if closed_trades.is_empty() {
        0.0
    } else {
        (winning_trades as f64 / closed_trades.len() as f64) * 100.0
    };
    
    Ok(PnlSummary {
        open_positions_count: open_positions.len(),
        closed_trades_count: closed_trades.len(),
        total_unrealized_pnl,
        total_unrealized_pnl_percent,
        total_realized_pnl,
        total_realized_pnl_percent,
        total_pnl,
        winning_trades,
        losing_trades,
        win_rate,
    })
}

#[derive(Debug, Clone)]
pub struct PnlSummary {
    pub open_positions_count: usize,
    pub closed_trades_count: usize,
    pub total_unrealized_pnl: f64,
    pub total_unrealized_pnl_percent: f64,
    pub total_realized_pnl: f64,
    pub total_realized_pnl_percent: f64,
    pub total_pnl: f64,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
}

