use crate::database::models::{
  Attachment, AttachmentTypeEnum, Message, MessageStatus, MessageTypeEnum, NewAttachment,
};
use crate::services::message::MessageWithAttachmentRaw;
use crate::utils::custom_serde::*;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct AttachmentPayload {
  #[serde(default = "i32::default")]
  pub id: i32,
  pub url: String,
  #[serde(default = "AttachmentTypeEnum::default")]
  pub attachment_type: AttachmentTypeEnum,
}

impl From<Attachment> for AttachmentPayload {
  fn from(value: Attachment) -> Self {
    Self {
      id: value.id,
      url: value.url,
      attachment_type: value.attachment_type,
    }
  }
}

impl<'a> AttachmentPayload {
  pub fn into_new(&'a self, message_id: i32) -> NewAttachment<'a> {
    NewAttachment {
      url: &self.url,
      message_id,
      attachment_type: self.attachment_type.clone(),
    }
  }
}

// Request structure for sending a message
#[derive(Deserialize, ToSchema)]
pub struct SendMessageRequest {
  pub message_uuid: Uuid,
  pub group_id: i32,
  pub content: Option<String>,
  #[serde(default = "MessageTypeEnum::default")]
  pub message_type: MessageTypeEnum,
  pub attachments: Option<Vec<AttachmentPayload>>,
}

impl SendMessageResponse {
  pub fn set_attachment(&mut self, attachments: Vec<AttachmentPayload>) {
    self.attachments = Some(attachments)
  }
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
  pub attachments: Option<Vec<AttachmentPayload>>,
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
      attachments: None,
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
  pub attachments: Option<Vec<AttachmentPayload>>,
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
      attachments: None,
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

#[derive(Queryable, Serialize, Debug, Clone, ToSchema)]
pub struct MessageWithUser {
  pub message_uuid: Uuid,
  pub id: i32,
  pub content: Option<String>,
  pub message_type: MessageTypeEnum,
  pub attachments: Option<Vec<AttachmentPayload>>,
  pub status: MessageStatus,
  #[serde(serialize_with = "serialize_naive_datetime")]
  pub created_at: NaiveDateTime,
  #[serde(serialize_with = "serialize_naive_datetime_option")]
  pub updated_at: Option<NaiveDateTime>,
  pub user_id: i32,
  pub user_name: String,
}

impl From<MessageWithAttachmentRaw> for MessageWithUser {
  fn from(value: MessageWithAttachmentRaw) -> Self {
    Self {
      message_uuid: value.message_uuid,
      id: value.id,
      content: value.content,
      message_type: value.message_type,
      attachments: None,
      status: value.status,
      created_at: value.created_at,
      updated_at: value.updated_at,
      user_id: value.user_id,
      user_name: value.user_name,
    }
  }
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
