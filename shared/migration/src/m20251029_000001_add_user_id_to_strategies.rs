use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Strategies::Table)
                    .add_column(
                        ColumnDef::new(Strategies::TelegramId)
                            .text()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Strategies::Table)
                    .drop_column(Strategies::TelegramId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Strategies {
    Table,
    Id,
    TelegramId,
}

