use serde::{Deserialize, Serialize, Serializer};

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


// for get list message content by click at any joined gr (gr id)
use chrono::NaiveDateTime;
use diesel::{Queryable};

// Request structure
#[derive(Deserialize)]
pub struct GetMessagesRequest {
    pub group_id: i32,
}

fn serialize_naive_datetime<S>(datetime: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&s)
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: i32,
    pub content: Option<String>,
    pub message_type: String,
    #[serde(serialize_with = "serialize_naive_datetime")]
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub user_name: String,
}

// Full response structure containing a list of messages
#[derive(Serialize)]
pub struct GetMessagesResponse {
    pub messages: Vec<MessageResponse>,
}

// #[derive(Queryable, Serialize)]
// pub struct MessageWithUser {
//     pub id: i32,
//     pub content: Option<String>,
//     pub message_type: String,
//     pub created_at: NaiveDateTime,
//     pub user_id: i32,
//     pub user_name: String,
//     pub user_avatar: Option<String>,
// }

#[derive(Queryable, Serialize, Debug)]
pub struct MessageWithUser {
    pub id: i32,
    pub content: Option<String>,
    pub message_type: String,
    #[serde(serialize_with = "serialize_naive_datetime")]
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub user_name: String,
}