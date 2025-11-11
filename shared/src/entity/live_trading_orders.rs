//! `SeaORM` Entity, @generated manually

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "live_trading_orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub user_id: i64,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub strategy_id: Option<u64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub strategy_name: Option<String>,
    pub exchange: String,
    pub pair: String,
    pub side: String, // "buy" or "sell"
    pub signal_type: String, // "buy", "sell", "hold"
    #[sea_orm(column_type = "Text")]
    pub price: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub confidence: Option<String>, // 0.00 to 1.00
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub timeframe: Option<String>, // "1m", "5m", "1h", etc.
    pub status: String, // "signal", "executed", "cancelled", "failed"
    #[sea_orm(column_type = "Text", nullable)]
    pub external_order_id: Option<String>, // Order ID from exchange if executed
    #[sea_orm(column_type = "Text", nullable)]
    pub executed_price: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub executed_quantity: Option<String>,
    pub executed_at: Option<DateTimeUtc>,
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

