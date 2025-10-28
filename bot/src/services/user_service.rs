use anyhow::Result;
use std::sync::Arc;
use sea_orm::prelude::DatabaseConnection;
use shared::entity::users;
use crate::repositories::user_repository::UserRepository;

pub struct UserService {
    repo: UserRepository,
}

impl UserService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let repo = UserRepository::new(db);
        Self { repo }
    }

    /// Lấy thông tin user hiện tại
    pub async fn get_current_user(&self, user_id: i64) -> Result<Option<users::Model>> {
        self.repo.find_by_id(user_id).await
    }

    /// Lấy username từ user_id
    pub async fn get_username_from_user(&self, user_id: i64) -> Result<Option<String>> {
        match self.repo.find_by_id(user_id).await? {
            Some(user) => Ok(user.username),
            None => Ok(None),
        }
    }

    /// Tạo user mới
    pub async fn create_user(&self, active_model: users::ActiveModel) -> Result<users::Model> {
        self.repo.create(active_model).await
    }

    /// Cập nhật thông tin user
    pub async fn update_user(&self, id: i64, active_model: users::ActiveModel) -> Result<users::Model> {
        self.repo.update(id, active_model).await
    }

    /// Kiểm tra user có tồn tại không
    pub async fn user_exists(&self, user_id: i64) -> Result<bool> {
        let user = self.repo.find_by_id(user_id).await?;
        Ok(user.is_some())
    }

    /// Lấy tất cả users
    pub async fn list_users(&self) -> Result<Vec<users::Model>> {
        self.repo.list_all().await
    }

    /// Đếm số lượng users
    pub async fn count_users(&self) -> Result<usize> {
        self.repo.count().await
    }

    /// Lấy user theo username
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<users::Model>> {
        self.repo.find_by_username(username).await
    }
}

