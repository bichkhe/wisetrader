use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename table from live_trading_orders to live_trading_signals using raw SQL
        let backend = manager.get_database_backend();
        manager
            .get_connection()
            .execute(sea_orm::Statement::from_string(
                backend,
                "RENAME TABLE `live_trading_orders` TO `live_trading_signals`".to_string(),
            ))
            .await?;

        // Rename foreign key
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_live_trading_orders_user")
                    .table(LiveTradingSignals::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_live_trading_signals_user")
                    .from(LiveTradingSignals::Table, LiveTradingSignals::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Rename indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_user_created")
                    .table(LiveTradingSignals::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_created")
                    .table(LiveTradingSignals::Table)
                    .col(LiveTradingSignals::UserId)
                    .col(LiveTradingSignals::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_user_status")
                    .table(LiveTradingSignals::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_status")
                    .table(LiveTradingSignals::Table)
                    .col(LiveTradingSignals::UserId)
                    .col(LiveTradingSignals::Status)
                    .to_owned(),
            )
            .await?;

        // Add new columns for better signal tracking
        manager
            .alter_table(
                Table::alter()
                    .table(LiveTradingSignals::Table)
                    .add_column(ColumnDef::new(LiveTradingSignals::CandleTimestamp).timestamp().null()) // Timestamp of candle that generated signal
                    .add_column(ColumnDef::new(LiveTradingSignals::IndicatorValues).text().null()) // JSON string: {"rsi": 30.5, "macd": 0.02, ...}
                    .add_column(ColumnDef::new(LiveTradingSignals::TelegramMessageId).big_integer().null()) // Telegram message ID
                    .add_column(ColumnDef::new(LiveTradingSignals::RelatedSignalId).big_unsigned().null()) // Link to previous signal in sequence
                    .to_owned(),
            )
            .await?;

        // Add index for candle_timestamp for time-based queries
        manager
            .create_index(
                Index::create()
                    .name("idx_candle_timestamp")
                    .table(LiveTradingSignals::Table)
                    .col(LiveTradingSignals::CandleTimestamp)
                    .to_owned(),
            )
            .await?;

        // Add index for related_signal_id for signal chain tracking
        manager
            .create_index(
                Index::create()
                    .name("idx_related_signal")
                    .table(LiveTradingSignals::Table)
                    .col(LiveTradingSignals::RelatedSignalId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove new columns
        manager
            .alter_table(
                Table::alter()
                    .table(LiveTradingSignals::Table)
                    .drop_column(LiveTradingSignals::CandleTimestamp)
                    .drop_column(LiveTradingSignals::IndicatorValues)
                    .drop_column(LiveTradingSignals::TelegramMessageId)
                    .drop_column(LiveTradingSignals::RelatedSignalId)
                    .to_owned(),
            )
            .await?;

        // Drop new indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_candle_timestamp")
                    .table(LiveTradingSignals::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_related_signal")
                    .table(LiveTradingSignals::Table)
                    .to_owned(),
            )
            .await?;

        // Rename table back using raw SQL
        let backend = manager.get_database_backend();
        manager
            .get_connection()
            .execute(sea_orm::Statement::from_string(
                backend,
                "RENAME TABLE `live_trading_signals` TO `live_trading_orders`".to_string(),
            ))
            .await?;

        // Rename foreign key back
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_live_trading_signals_user")
                    .table(LiveTradingOrders::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_live_trading_orders_user")
                    .from(LiveTradingOrders::Table, LiveTradingOrders::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum LiveTradingOrders {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum LiveTradingSignals {
    Table,
    UserId,
    CreatedAt,
    Status,
    CandleTimestamp,
    IndicatorValues,
    TelegramMessageId,
    RelatedSignalId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

