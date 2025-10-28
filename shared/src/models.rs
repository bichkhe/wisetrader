use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    pub subscription_tier: String,
    pub subscription_expires: Option<DateTime<Utc>>,
    pub live_trading_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Strategy {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub repo_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserStrategy {
    pub id: i32,
    pub user_id: i64,
    pub strategy_id: i32,
    pub params: Option<serde_json::Value>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Signal {
    pub id: i64,
    pub strategy_id: i32,
    pub payload: serde_json::Value,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: i64,
    pub user_id: i64,
    pub exchange: Option<String>,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub qty: Option<String>,
    pub price: Option<String>,
    pub status: Option<String>,
    pub external_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingPlan {
    pub id: String,
    pub name: String,
    pub price_monthly_usd: String,
    pub duration_days: Option<i32>,
    pub features: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    pub plan: BillingPlan,
    pub expires_at: Option<DateTime<Utc>>,
    pub active: bool,
}

