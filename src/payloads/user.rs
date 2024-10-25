use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct NewUserRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub user_id: i32,
    pub username: String,
    pub user_code: String,
}
