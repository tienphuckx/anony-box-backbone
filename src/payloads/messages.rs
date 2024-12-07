use crate::database::models::{Message, MessageStatus, MessageTypeEnum};
use crate::utils::custom_serde::*;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
// Request structure for sending a message

#[derive(Deserialize, ToSchema)]
pub struct SendMessageRequest {
  pub message_uuid: Uuid,
  pub group_id: i32,
  pub content: String,
  #[serde(default = "MessageTypeEnum::default")]
  pub message_type: MessageTypeEnum,
}

// Response structure for sending a message
#[derive(Serialize, ToSchema)]
pub struct SendMessageResponse {
  pub message_uuid: Uuid,
  pub message_id: i32,
  pub content: String,
  pub message_type: MessageTypeEnum,
  pub status: MessageStatus,
  #[serde(serialize_with = "serialize_with_date_time_utc")]
  pub created_at: DateTime<Utc>,
}

impl From<Message> for SendMessageResponse {
  fn from(value: Message) -> Self {
    Self {
      message_uuid: value.message_uuid,
      message_id: value.id,
      content: value.content.unwrap_or_default(),
      message_type: value.message_type,
      status: value.status,
      created_at: value.created_at.and_utc(),
    }
  }
}

// for get list message content by click at any joined gr (gr id)
use chrono::NaiveDateTime;
use diesel::Queryable;

use super::common::OrderBy;

#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
  pub id: i32,
  pub content: Option<String>,
  pub message_type: MessageTypeEnum,
  pub status: MessageStatus,
  #[serde(serialize_with = "serialize_naive_datetime")]
  pub created_at: NaiveDateTime,
  #[serde(serialize_with = "serialize_naive_datetime_option")]
  pub updated_at: Option<NaiveDateTime>,
  pub user_id: i32,
  pub user_name: String,
}

impl From<Message> for MessageResponse {
  fn from(value: Message) -> Self {
    Self {
      id: value.id,
      content: value.content,
      message_type: value.message_type,
      status: value.status,
      created_at: value.created_at,
      updated_at: value.updated_at,
      user_id: value.user_id,
      user_name: "".into(),
    }
  }
}

// Full response structure containing a list of messages
#[derive(Serialize)]
pub struct GetMessagesResponse {
  pub messages: Vec<MessageResponse>,
}

#[derive(Queryable, Serialize, Debug, ToSchema)]
pub struct MessageWithUser {
  pub message_uuid: Uuid,
  pub id: i32,
  pub content: Option<String>,
  pub message_type: MessageTypeEnum,
  pub status: MessageStatus,
  #[serde(serialize_with = "serialize_naive_datetime")]
  pub created_at: NaiveDateTime,
  #[serde(serialize_with = "serialize_naive_datetime_option")]
  pub updated_at: Option<NaiveDateTime>,
  pub user_id: i32,
  pub user_name: String,
}

#[derive(Deserialize)]
pub struct MessageFilterParams {
  pub message_type: Option<MessageTypeEnum>,
  pub content: Option<String>,
  pub status: Option<MessageStatus>,
  #[serde(
    deserialize_with = "deserialize_with_naive_date_option",
    default = "Option::default"
  )]
  pub from_date: Option<NaiveDate>,
  #[serde(
    deserialize_with = "deserialize_with_naive_date_option",
    default = "Option::default"
  )]
  pub to_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct MessageSortParams {
  pub created_at_sort: Option<OrderBy>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateMessage {
  pub content: Option<String>,
  pub message_type: Option<MessageTypeEnum>,
}
