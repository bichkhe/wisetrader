use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LiveTradingOrders::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LiveTradingOrders::Id).big_unsigned().auto_increment().primary_key())
                    .col(ColumnDef::new(LiveTradingOrders::UserId).big_integer().not_null())
                    .col(ColumnDef::new(LiveTradingOrders::StrategyId).big_unsigned().null())
                    .col(ColumnDef::new(LiveTradingOrders::StrategyName).text().null())
                    .col(ColumnDef::new(LiveTradingOrders::Exchange).string().not_null()) // "binance" or "okx"
                    .col(ColumnDef::new(LiveTradingOrders::Pair).string().not_null()) // "BTC/USDT"
                    .col(ColumnDef::new(LiveTradingOrders::Side).string().not_null()) // "buy" or "sell"
                    .col(ColumnDef::new(LiveTradingOrders::SignalType).string().not_null()) // "buy", "sell", "hold"
                    .col(ColumnDef::new(LiveTradingOrders::Price).decimal_len(20, 8).not_null())
                    .col(ColumnDef::new(LiveTradingOrders::Confidence).decimal_len(5, 2).null()) // 0.00 to 1.00
                    .col(ColumnDef::new(LiveTradingOrders::Reason).text().null())
                    .col(ColumnDef::new(LiveTradingOrders::Timeframe).string().null()) // "1m", "5m", "1h", etc.
                    .col(ColumnDef::new(LiveTradingOrders::Status).string().not_null().default("signal")) // "signal", "executed", "cancelled", "failed"
                    .col(ColumnDef::new(LiveTradingOrders::ExternalOrderId).text().null()) // Order ID from exchange if executed
                    .col(ColumnDef::new(LiveTradingOrders::ExecutedPrice).decimal_len(20, 8).null())
                    .col(ColumnDef::new(LiveTradingOrders::ExecutedQuantity).decimal_len(20, 8).null())
                    .col(ColumnDef::new(LiveTradingOrders::ExecutedAt).timestamp().null())
                    .col(ColumnDef::new(LiveTradingOrders::CreatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(LiveTradingOrders::UpdatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")))
                    .index(
                        Index::create()
                            .name("idx_user_created")
                            .table(LiveTradingOrders::Table)
                            .col(LiveTradingOrders::UserId)
                            .col(LiveTradingOrders::CreatedAt)
                    )
                    .index(
                        Index::create()
                            .name("idx_user_status")
                            .table(LiveTradingOrders::Table)
                            .col(LiveTradingOrders::UserId)
                            .col(LiveTradingOrders::Status)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_live_trading_orders_user")
                            .from(LiveTradingOrders::Table, LiveTradingOrders::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LiveTradingOrders::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LiveTradingOrders {
    Table,
    Id,
    UserId,
    StrategyId,
    StrategyName,
    Exchange,
    Pair,
    Side,
    SignalType,
    Price,
    Confidence,
    Reason,
    Timeframe,
    Status,
    ExternalOrderId,
    ExecutedPrice,
    ExecutedQuantity,
    ExecutedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

