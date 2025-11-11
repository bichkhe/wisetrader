//! PnL Command - View profit and loss statistics

use std::sync::Arc;
use anyhow::Result;
use teloxide::prelude::*;
use sea_orm::EntityTrait;
use crate::state::AppState;
use crate::i18n;
use shared::entity::users;
use crate::services::position_service;

/// Handler for /pnl command to view profit and loss
pub async fn handle_pnl(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let from = msg.from.unwrap();
    let telegram_id = from.id.0 as i64;
    
    // Get user locale
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    // Get P&L summary
    let pnl_summary = match position_service::calculate_pnl_summary(state.db.as_ref(), telegram_id).await {
        Ok(summary) => summary,
        Err(e) => {
            let error_msg = if locale == "vi" {
                format!("❌ Lỗi khi tính toán P&L: {}", e)
            } else {
                format!("❌ Error calculating P&L: {}", e)
            };
            bot.send_message(msg.chat.id, error_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            return Ok(());
        }
    };
    
    // Get open positions
    let open_positions = position_service::get_open_positions(state.db.as_ref(), telegram_id).await?;
    
    // Get recent trades
    let recent_trades = position_service::get_user_trades(state.db.as_ref(), telegram_id, Some(10)).await?;
    
    // Format message
    let message = format_pnl_message(&pnl_summary, &open_positions, &recent_trades, locale);
    
    bot.send_message(msg.chat.id, message)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
    Ok(())
}

fn format_pnl_message(
    summary: &position_service::PnlSummary,
    open_positions: &[shared::entity::positions::Model],
    recent_trades: &[shared::entity::trades::Model],
    locale: &str,
) -> String {
    if locale == "vi" {
        format!(
            "📊 <b>Profit & Loss (P&L) Summary</b>\n\n\
            <b>📈 Vị thế đang mở:</b> {}\n\
            <b>💰 Tổng Unrealized P&L:</b> <code>{:.2} USDT</code> ({:.2}%)\n\n\
            <b>✅ Giao dịch đã đóng:</b> {}\n\
            <b>💵 Tổng Realized P&L:</b> <code>{:.2} USDT</code> ({:.2}%)\n\n\
            <b>📊 Tổng P&L:</b> <code>{:.2} USDT</code>\n\
            <b>🎯 Win Rate:</b> <code>{:.1}%</code> ({} thắng / {} thua)\n\n\
            {}\
            {}\
            \n\
            <i>💡 Unrealized P&L: Lời/lỗ chưa thực hiện từ vị thế đang mở\n\
            Realized P&L: Lời/lỗ đã thực hiện từ giao dịch đã đóng</i>",
            summary.open_positions_count,
            summary.total_unrealized_pnl,
            summary.total_unrealized_pnl_percent,
            summary.closed_trades_count,
            summary.total_realized_pnl,
            summary.total_realized_pnl_percent,
            summary.total_pnl,
            summary.win_rate,
            summary.winning_trades,
            summary.losing_trades,
            format_open_positions(open_positions, locale),
            format_recent_trades(recent_trades, locale),
        )
    } else {
        format!(
            "📊 <b>Profit & Loss (P&L) Summary</b>\n\n\
            <b>📈 Open Positions:</b> {}\n\
            <b>💰 Total Unrealized P&L:</b> <code>{:.2} USDT</code> ({:.2}%)\n\n\
            <b>✅ Closed Trades:</b> {}\n\
            <b>💵 Total Realized P&L:</b> <code>{:.2} USDT</code> ({:.2}%)\n\n\
            <b>📊 Total P&L:</b> <code>{:.2} USDT</code>\n\
            <b>🎯 Win Rate:</b> <code>{:.1}%</code> ({} wins / {} losses)\n\n\
            {}\
            {}\
            \n\
            <i>💡 Unrealized P&L: Unrealized profit/loss from open positions\n\
            Realized P&L: Realized profit/loss from closed trades</i>",
            summary.open_positions_count,
            summary.total_unrealized_pnl,
            summary.total_unrealized_pnl_percent,
            summary.closed_trades_count,
            summary.total_realized_pnl,
            summary.total_realized_pnl_percent,
            summary.total_pnl,
            summary.win_rate,
            summary.winning_trades,
            summary.losing_trades,
            format_open_positions(open_positions, locale),
            format_recent_trades(recent_trades, locale),
        )
    }
}

fn format_open_positions(positions: &[shared::entity::positions::Model], locale: &str) -> String {
    if positions.is_empty() {
        return String::new();
    }
    
    let title = if locale == "vi" {
        "<b>📈 Vị thế đang mở:</b>\n"
    } else {
        "<b>📈 Open Positions:</b>\n"
    };
    
    let mut text = title.to_string();
    
    for position in positions.iter().take(5) {
        let unrealized_pnl: f64 = position.unrealized_pnl.parse().unwrap_or(0.0);
        let unrealized_pnl_percent: f64 = position.unrealized_pnl_percent.parse().unwrap_or(0.0);
        let entry_price: f64 = position.entry_price.parse().unwrap_or(0.0);
        let current_price: f64 = position.current_price.as_ref()
            .and_then(|p| p.parse().ok())
            .unwrap_or(entry_price);
        let quantity: f64 = position.quantity.parse().unwrap_or(0.0);
        
        let pnl_emoji = if unrealized_pnl >= 0.0 { "🟢" } else { "🔴" };
        
        text.push_str(&format!(
            "{} <b>{}</b> | Entry: <code>{:.4}</code> | Current: <code>{:.4}</code>\n\
            Quantity: <code>{:.6}</code> | P&L: <code>{:.2} USDT</code> ({:.2}%)\n\n",
            pnl_emoji,
            position.pair,
            entry_price,
            current_price,
            quantity,
            unrealized_pnl,
            unrealized_pnl_percent
        ));
    }
    
    if positions.len() > 5 {
        text.push_str(&format!(
            "... và {} vị thế khác\n\n",
            positions.len() - 5
        ));
    }
    
    text
}

fn format_recent_trades(trades: &[shared::entity::trades::Model], locale: &str) -> String {
    if trades.is_empty() {
        return String::new();
    }
    
    let title = if locale == "vi" {
        "<b>✅ Giao dịch gần đây:</b>\n"
    } else {
        "<b>✅ Recent Trades:</b>\n"
    };
    
    let mut text = title.to_string();
    
    for trade in trades.iter().take(5) {
        let pnl: f64 = trade.pnl.parse().unwrap_or(0.0);
        let pnl_percent: f64 = trade.pnl_percent.parse().unwrap_or(0.0);
        let entry_price: f64 = trade.entry_price.parse().unwrap_or(0.0);
        let exit_price: f64 = trade.exit_price.parse().unwrap_or(0.0);
        let quantity: f64 = trade.quantity.parse().unwrap_or(0.0);
        
        let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
        
        text.push_str(&format!(
            "{} <b>{}</b> | Entry: <code>{:.4}</code> | Exit: <code>{:.4}</code>\n\
            Quantity: <code>{:.6}</code> | P&L: <code>{:.2} USDT</code> ({:.2}%)\n\n",
            pnl_emoji,
            trade.pair,
            entry_price,
            exit_price,
            quantity,
            pnl,
            pnl_percent
        ));
    }
    
    if trades.len() > 5 {
        text.push_str(&format!(
            "... và {} giao dịch khác\n\n",
            trades.len() - 5
        ));
    }
    
    text
}

