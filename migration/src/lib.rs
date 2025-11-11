pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20251029_000001_add_user_id_to_strategies;
mod m20251103_073712_add_content_to_strategies;
mod m20251106_000001_create_exchange_tokens;
mod m20251107_000001_create_live_trading_orders;
mod m20251108_000001_create_positions_and_trades;
mod m20251109_000001_rename_live_trading_orders_to_signals;
mod m20251110_000001_create_live_trading_sessions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20251029_000001_add_user_id_to_strategies::Migration),
            Box::new(m20251103_073712_add_content_to_strategies::Migration),
            Box::new(m20251106_000001_create_exchange_tokens::Migration),
                Box::new(m20251107_000001_create_live_trading_orders::Migration),
                Box::new(m20251108_000001_create_positions_and_trades::Migration),
                Box::new(m20251109_000001_rename_live_trading_orders_to_signals::Migration),
                Box::new(m20251110_000001_create_live_trading_sessions::Migration),
        ]
    }
}
