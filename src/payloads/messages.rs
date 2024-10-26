use serde::{Deserialize, Serialize};

// Request structure for sending a message
#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub user_id: i32,
    pub group_id: i32,
    pub content: String,
    pub message_type: String,  // Example values: "text"
}

// Response structure for sending a message
#[derive(Serialize)]
pub struct SendMessageResponse {
    pub message_id: i32,
    pub content: String,
    pub message_type: String,
    pub created_at: String,
}
