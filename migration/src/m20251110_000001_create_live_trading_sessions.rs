use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LiveTradingSessions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LiveTradingSessions::Id).big_unsigned().auto_increment().primary_key())
                    .col(ColumnDef::new(LiveTradingSessions::UserId).big_integer().not_null())
                    .col(ColumnDef::new(LiveTradingSessions::StrategyId).big_unsigned().null())
                    .col(ColumnDef::new(LiveTradingSessions::StrategyName).text().null())
                    .col(ColumnDef::new(LiveTradingSessions::Exchange).string().not_null()) // "binance" or "okx"
                    .col(ColumnDef::new(LiveTradingSessions::Pair).string().not_null()) // "BTC/USDT"
                    .col(ColumnDef::new(LiveTradingSessions::Timeframe).string().null()) // "1m", "5m", "1h", etc.
                    .col(ColumnDef::new(LiveTradingSessions::Status).string().not_null().default("active")) // "active", "stopped", "error"
                    .col(ColumnDef::new(LiveTradingSessions::StartedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(LiveTradingSessions::StoppedAt).timestamp().null())
                    .col(ColumnDef::new(LiveTradingSessions::CreatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(LiveTradingSessions::UpdatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")))
                    .index(
                        Index::create()
                            .name("idx_user_status")
                            .table(LiveTradingSessions::Table)
                            .col(LiveTradingSessions::UserId)
                            .col(LiveTradingSessions::Status)
                    )
                    .index(
                        Index::create()
                            .name("idx_user_started")
                            .table(LiveTradingSessions::Table)
                            .col(LiveTradingSessions::UserId)
                            .col(LiveTradingSessions::StartedAt)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_live_trading_sessions_user")
                            .from(LiveTradingSessions::Table, LiveTradingSessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_live_trading_sessions_strategy")
                            .from(LiveTradingSessions::Table, LiveTradingSessions::StrategyId)
                            .to(Strategies::Table, Strategies::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LiveTradingSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LiveTradingSessions {
    Table,
    Id,
    UserId,
    StrategyId,
    StrategyName,
    Exchange,
    Pair,
    Timeframe,
    Status,
    StartedAt,
    StoppedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Strategies {
    Table,
    Id,
}

