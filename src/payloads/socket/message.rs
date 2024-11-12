use crate::{
  database::models::{Message, NewMessage},
  utils::minors::custom_serde::*,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ## Authentication Result structure
///
/// ### Properties:
/// - `status_code`: status code for result
///   - 0 : Authentication successfully
///   - 1 : Authentication timeout
///   - 2 : UnSupport authenticated message type
///   - 3 : User does not have permission to access this group
///   - 4 : User token is expired or not found
///   - 5 : Failed to get user from user code
///
/// - `message`: short message for result
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AuthenticateResult {
  pub status_code: i32,
  pub message: String,
}
impl AuthenticateResult {
  pub fn new(status_code: i32, message: &str) -> Self {
    Self {
      status_code,
      message: message.into(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SMessageType {
  Authenticate(String),
  AuthenticateResponse(AuthenticateResult),
  Send(SNewMessage),
  Receive(SMessageContent),
  Edit(SMessageContent),
  Delete(Vec<i32>),
  UnSupportMessage(String),
}

#[derive(Serialize, Clone, Deserialize, Debug, PartialEq)]

pub struct SMessageContent {
  pub message_uuid: Uuid,
  pub user_id: i32,
  pub group_id: i32,
  pub content: String,
  #[serde(
    serialize_with = "serialize_with_date_time_utc",
    deserialize_with = "deserialize_with_date_time_utc"
  )]
  pub created_at: DateTime<Utc>,
  pub status: SMessageStatus,
}
impl From<Message> for SMessageContent {
  fn from(value: Message) -> Self {
    Self {
      message_uuid: value.message_uuid,
      user_id: value.user_id,
      group_id: value.group_id,
      content: value.content.unwrap_or_default(),
      created_at: value.created_at.and_utc(),
      status: SMessageStatus::Sent,
    }
  }
}

#[derive(Serialize, Clone, Deserialize, Debug, PartialEq)]
pub struct SNewMessage {
  message_uuid: Uuid,
  pub content: String,
}

impl<'a> SNewMessage {
  pub fn build_new_message(&'a self, user_id: i32, group_id: i32) -> NewMessage<'a> {
    NewMessage {
      message_uuid: self.message_uuid,
      user_id,
      group_id,
      content: Some(&self.content),
      created_at: Utc::now().naive_utc(),
      message_type: crate::database::models::MessageTypeEnum::TEXT,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SMessageStatus {
  Sent,
  InProgress,
  Error,
}
