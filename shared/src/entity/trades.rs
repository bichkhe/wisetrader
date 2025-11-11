//! `SeaORM` Entity, @generated manually

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "trades")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub user_id: i64,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub position_id: Option<u64>,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub buy_order_id: Option<u64>,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub sell_order_id: Option<u64>,
    #[sea_orm(column_type = "BigUnsigned", nullable)]
    pub strategy_id: Option<u64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub strategy_name: Option<String>,
    pub exchange: String,
    pub pair: String,
    #[sea_orm(column_type = "Text")]
    pub entry_price: String,
    #[sea_orm(column_type = "Text")]
    pub exit_price: String,
    #[sea_orm(column_type = "Text")]
    pub quantity: String,
    #[sea_orm(column_type = "Text")]
    pub entry_value: String,
    #[sea_orm(column_type = "Text")]
    pub exit_value: String,
    #[sea_orm(column_type = "Text")]
    pub pnl: String,
    #[sea_orm(column_type = "Text")]
    pub pnl_percent: String,
    pub entry_time: Option<DateTimeUtc>,
    pub exit_time: Option<DateTimeUtc>,
    #[sea_orm(column_type = "BigInteger", nullable)]
    pub duration: Option<i64>, // Duration in seconds
    pub created_at: Option<DateTimeUtc>,
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

