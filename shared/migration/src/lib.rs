pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20251029_000001_add_user_id_to_strategies;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20251029_000001_add_user_id_to_strategies::Migration),
        ]
    }
}
