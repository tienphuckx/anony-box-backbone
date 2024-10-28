use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct NewUserRequest {
    pub username: String,
}

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub user_id: i32,
    pub username: String,
    pub user_code: String,
}
