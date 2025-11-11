//! `SeaORM` Entity, @generated manually

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "live_trading_sessions")]
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
    #[sea_orm(column_type = "Text", nullable)]
    pub timeframe: Option<String>, // "1m", "5m", "1h", etc.
    pub status: String, // "active", "stopped", "error"
    pub started_at: Option<DateTimeUtc>,
    pub stopped_at: Option<DateTimeUtc>,
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
    #[sea_orm(
        belongs_to = "super::strategies::Entity",
        from = "Column::StrategyId",
        to = "super::strategies::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Strategies,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl Related<super::strategies::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Strategies.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

