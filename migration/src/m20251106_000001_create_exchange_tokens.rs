use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExchangeTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ExchangeTokens::Id).big_unsigned().auto_increment().primary_key())
                    .col(ColumnDef::new(ExchangeTokens::UserId).big_integer().not_null())
                    .col(ColumnDef::new(ExchangeTokens::Exchange).string().not_null()) // "binance" or "okx"
                    .col(ColumnDef::new(ExchangeTokens::ApiKey).text().not_null())
                    .col(ColumnDef::new(ExchangeTokens::ApiSecret).text().not_null())
                    .col(ColumnDef::new(ExchangeTokens::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(ExchangeTokens::CreatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(ExchangeTokens::UpdatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")))
                    .index(
                        Index::create()
                            .name("idx_user_exchange")
                            .table(ExchangeTokens::Table)
                            .col(ExchangeTokens::UserId)
                            .col(ExchangeTokens::Exchange)
                            .unique()
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_exchange_tokens_user")
                            .from(ExchangeTokens::Table, ExchangeTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExchangeTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ExchangeTokens {
    Table,
    Id,
    UserId,
    Exchange,
    ApiKey,
    ApiSecret,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

