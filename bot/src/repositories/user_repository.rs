use anyhow::Result;
use sea_orm::prelude::*;
use std::sync::Arc;
use shared::entity::users;

pub struct UserRepository {
    db: Arc<DatabaseConnection>,
}

impl UserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, user_id: i64) -> Result<Option<users::Model>> {
        let user = users::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<users::Model>> {
        let user = users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .one(self.db.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn create(&self, active_model: users::ActiveModel) -> Result<users::Model> {
        let user = users::Entity::insert(active_model)
            .exec_with_returning(self.db.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn update(&self, id: i64, active_model: users::ActiveModel) -> Result<users::Model> {
        let user = users::Entity::update(active_model)
            .filter(users::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await?;
        Ok(user)
    }

    pub async fn list_all(&self) -> Result<Vec<users::Model>> {
        let users = users::Entity::find()
            .all(self.db.as_ref())
            .await?;
        Ok(users)
    }

    pub async fn count(&self) -> Result<usize> {
        let count = users::Entity::find()
            .count(self.db.as_ref())
            .await?;
        Ok(count as usize)
    }
}

