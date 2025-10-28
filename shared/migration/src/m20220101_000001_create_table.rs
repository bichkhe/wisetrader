use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create strategies table first (no dependencies)
        manager
            .create_table(
                Table::create()
                    .table(Strategies::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Strategies::Id).big_unsigned().auto_increment().primary_key())
                    .col(ColumnDef::new(Strategies::Name).text().null())
                    .col(ColumnDef::new(Strategies::Description).text().null())
                    .col(ColumnDef::new(Strategies::RepoRef).text().null())
                    .col(ColumnDef::new(Strategies::CreatedAt).timestamp().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .to_owned(),
            )
            .await?;

        // Create users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).big_integer().not_null().primary_key())
                    .col(ColumnDef::new(Users::Username).text().null())
                    .col(ColumnDef::new(Users::Language).text().null())
                    .col(ColumnDef::new(Users::CreatedAt).timestamp().null().default(Expr::cust("CURRENT_TIMESTAMP")))
                    .col(ColumnDef::new(Users::SubscriptionTier).text().null())
                    .col(ColumnDef::new(Users::SubscriptionExpires).timestamp().null())
                    .col(ColumnDef::new(Users::LiveTradingEnabled).boolean().default(false))
                    .col(ColumnDef::new(Users::TelegramId).text().null())
                    .col(ColumnDef::new(Users::Fullname).text().null())
                    .col(ColumnDef::new(Users::Points).big_unsigned().not_null().default(0))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Strategies::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Strategies {
    Table,
    Id,
    Name,
    Description,
    RepoRef,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    Language,
    CreatedAt,
    SubscriptionTier,
    SubscriptionExpires,
    LiveTradingEnabled,
    TelegramId,
    Fullname,
    Points,
}
