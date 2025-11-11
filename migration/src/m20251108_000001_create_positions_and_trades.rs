use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create positions table (vị thế đang mở)
        manager
            .create_table(
                Table::create()
                    .table(Positions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Positions::Id).big_unsigned().auto_increment().primary_key())
                    .col(ColumnDef::new(Positions::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Positions::OrderId).big_unsigned().null()) // Reference to live_trading_orders
                    .col(ColumnDef::new(Positions::StrategyId).big_unsigned().null())
                    .col(ColumnDef::new(Positions::StrategyName).text().null())
                    .col(ColumnDef::new(Positions::Exchange).string().not_null())
                    .col(ColumnDef::new(Positions::Pair).string().not_null())
                    .col(ColumnDef::new(Positions::Side).string().not_null()) // "buy" or "sell"
                    .col(ColumnDef::new(Positions::EntryPrice).decimal_len(20, 8).not_null())
                    .col(ColumnDef::new(Positions::Quantity).decimal_len(20, 8).not_null())
                    .col(ColumnDef::new(Positions::EntryValue).decimal_len(20, 8).not_null()) // entry_price * quantity
                    .col(ColumnDef::new(Positions::CurrentPrice).decimal_len(20, 8).null()) // Updated periodically
                    .col(ColumnDef::new(Positions::UnrealizedPnl).decimal_len(20, 8).default(0.0)) // Current P&L
                    .col(ColumnDef::new(Positions::UnrealizedPnlPercent).decimal_len(10, 4).default(0.0)) // P&L percentage
                    .col(ColumnDef::new(Positions::Status).string().not_null().default("open")) // "open", "closed"
                    .col(ColumnDef::new(Positions::EntryTime).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(Positions::CloseTime).timestamp().null())
                    .col(ColumnDef::new(Positions::CreatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(Positions::UpdatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")))
                    .index(
                        Index::create()
                            .name("idx_user_status")
                            .table(Positions::Table)
                            .col(Positions::UserId)
                            .col(Positions::Status)
                    )
                    .index(
                        Index::create()
                            .name("idx_user_pair")
                            .table(Positions::Table)
                            .col(Positions::UserId)
                            .col(Positions::Pair)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_positions_user")
                            .from(Positions::Table, Positions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        // Create trades table (giao dịch hoàn chỉnh - mua + bán)
        manager
            .create_table(
                Table::create()
                    .table(Trades::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Trades::Id).big_unsigned().auto_increment().primary_key())
                    .col(ColumnDef::new(Trades::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Trades::PositionId).big_unsigned().null()) // Reference to positions
                    .col(ColumnDef::new(Trades::BuyOrderId).big_unsigned().null()) // Reference to live_trading_orders (buy)
                    .col(ColumnDef::new(Trades::SellOrderId).big_unsigned().null()) // Reference to live_trading_orders (sell)
                    .col(ColumnDef::new(Trades::StrategyId).big_unsigned().null())
                    .col(ColumnDef::new(Trades::StrategyName).text().null())
                    .col(ColumnDef::new(Trades::Exchange).string().not_null())
                    .col(ColumnDef::new(Trades::Pair).string().not_null())
                    .col(ColumnDef::new(Trades::EntryPrice).decimal_len(20, 8).not_null()) // Buy price
                    .col(ColumnDef::new(Trades::ExitPrice).decimal_len(20, 8).not_null()) // Sell price
                    .col(ColumnDef::new(Trades::Quantity).decimal_len(20, 8).not_null())
                    .col(ColumnDef::new(Trades::EntryValue).decimal_len(20, 8).not_null()) // entry_price * quantity
                    .col(ColumnDef::new(Trades::ExitValue).decimal_len(20, 8).not_null()) // exit_price * quantity
                    .col(ColumnDef::new(Trades::Pnl).decimal_len(20, 8).not_null()) // Realized P&L
                    .col(ColumnDef::new(Trades::PnlPercent).decimal_len(10, 4).not_null()) // P&L percentage
                    .col(ColumnDef::new(Trades::EntryTime).timestamp().not_null())
                    .col(ColumnDef::new(Trades::ExitTime).timestamp().not_null())
                    .col(ColumnDef::new(Trades::Duration).big_integer().null()) // Duration in seconds
                    .col(ColumnDef::new(Trades::CreatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .index(
                        Index::create()
                            .name("idx_trades_user")
                            .table(Trades::Table)
                            .col(Trades::UserId)
                    )
                    .index(
                        Index::create()
                            .name("idx_trades_user_exit_time")
                            .table(Trades::Table)
                            .col(Trades::UserId)
                            .col(Trades::ExitTime)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_trades_user")
                            .from(Trades::Table, Trades::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Trades::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Positions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Positions {
    Table,
    Id,
    UserId,
    OrderId,
    StrategyId,
    StrategyName,
    Exchange,
    Pair,
    Side,
    EntryPrice,
    Quantity,
    EntryValue,
    CurrentPrice,
    UnrealizedPnl,
    UnrealizedPnlPercent,
    Status,
    EntryTime,
    CloseTime,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Trades {
    Table,
    Id,
    UserId,
    PositionId,
    BuyOrderId,
    SellOrderId,
    StrategyId,
    StrategyName,
    Exchange,
    Pair,
    EntryPrice,
    ExitPrice,
    Quantity,
    EntryValue,
    ExitValue,
    Pnl,
    PnlPercent,
    EntryTime,
    ExitTime,
    Duration,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

