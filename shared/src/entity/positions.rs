//! `SeaORM` Entity, @generated manually

use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "positions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub user_id: i64,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub order_id: Option<u64>,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub strategy_id: Option<u64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub strategy_name: Option<String>,
    pub exchange: String,
    pub pair: String,
    pub side: String, // "buy" or "sell"
    #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
    pub entry_price: Decimal,
    #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
    pub quantity: Decimal,
    #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
    pub entry_value: Decimal,
    #[sea_orm(column_type = "Decimal(Some((20, 8)))", nullable)]
    pub current_price: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
    pub unrealized_pnl: Decimal,
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub unrealized_pnl_percent: Decimal,
    pub status: String, // "open", "closed"
    pub entry_time: Option<DateTimeUtc>,
    pub close_time: Option<DateTimeUtc>,
    pub created_at: Option<DateTimeUtc>,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

