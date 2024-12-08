use crate::database::models::{Message, MessageStatus, MessageTypeEnum, NewMessage};

use crate::payloads::messages::{AttachmentPayload, UpdateMessage};
use crate::utils::custom_serde::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::ResultMessage;

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
///
#[allow(unused)]
pub enum AuthenticationStatusCode {
  Success,
  Timeout,
  UnsupportedMessageType,
  NoPermission,
  ExpireOrNotFound,
  Other,
}
impl Into<ResultMessage> for AuthenticationStatusCode {
  fn into(self) -> ResultMessage {
    match self {
      Self::Success => ResultMessage::new(0, "Authenticated Successfully"),
      Self::Timeout => ResultMessage::new(1, "Authentication Timeout"),
      Self::UnsupportedMessageType => {
        ResultMessage::new(2, "Only supports authenticated text message type")
      }
      Self::NoPermission => {
        ResultMessage::new(3, "User does not have permission to access this group")
      }
      Self::ExpireOrNotFound => ResultMessage::new(4, "User token is expired or not found"),
      Self::Other => ResultMessage::new(5, "Failed to get user from user code"),
    }
  }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessagesData {
  pub group_id: i32,
  pub message_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SMessageType {
  Authenticate(String),
  AuthenticateResponse(ResultMessage),

  SubscribeGroup(i32),
  SubscribeGroupResponse(ResultMessage),

  Send(SNewMessage),
  Receive(SMessageContent),

  EditMessage(SMessageEdit),
  EditMessageResponse(ResultMessage),
  EditMessageData(SMessageContent),

  DeleteMessage(MessagesData),
  DeleteMessageEvent(MessagesData),
  DeleteMessageResponse(ResultMessage),

  SeenMessages(MessagesData),
  SeenMessagesEvent(MessagesData),
  SeenMessagesResponse(ResultMessage),

  UnSupportMessage(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SMessageContent {
  pub message_uuid: Uuid,
  pub message_id: i32,
  pub user_id: i32,
  pub group_id: i32,
  pub content: String,
  pub username: Option<String>,
  pub message_type: MessageTypeEnum,
  pub attachments: Option<Vec<AttachmentPayload>>,
  #[serde(
    serialize_with = "serialize_with_date_time_utc",
    deserialize_with = "deserialize_with_date_time_utc"
  )]
  pub created_at: DateTime<Utc>,
  #[serde(
    serialize_with = "serialize_with_date_time_utc_option",
    deserialize_with = "deserialize_with_date_time_utc_option"
  )]
  pub updated_at: Option<DateTime<Utc>>,
  pub status: SMessageStatus,
}
impl From<Message> for SMessageContent {
  fn from(value: Message) -> Self {
    Self {
      message_uuid: value.message_uuid,
      message_id: value.id,
      user_id: value.user_id,
      username: None,
      group_id: value.group_id,
      message_type: value.message_type,
      attachments: None,
      content: value.content.unwrap_or_default(),
      created_at: value.created_at.and_utc(),
      updated_at: value.updated_at.map(|data| data.and_utc()),
      status: SMessageStatus::from(value.status),
    }
  }
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct SNewMessage {
  pub message_uuid: Uuid,
  pub group_id: i32,
  pub message_type: Option<MessageTypeEnum>,
  pub content: Option<String>,
  pub attachments: Option<Vec<AttachmentPayload>>,
}

impl<'a> SNewMessage {
  pub fn build_new_message(&'a self, user_id: i32) -> NewMessage<'a> {
    let message_type = if self.message_type.is_some() {
      self.message_type.clone().unwrap()
    } else {
      MessageTypeEnum::TEXT
    };
    NewMessage {
      message_uuid: self.message_uuid,
      user_id,
      group_id: self.group_id,
      content: self.content.as_ref(),
      status: MessageStatus::Sent,
      created_at: Utc::now().naive_utc(),
      message_type,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SMessageEdit {
  pub message_id: i32,
  pub group_id: i32,
  pub content: Option<String>,
  pub message_type: Option<MessageTypeEnum>,
}
impl Into<UpdateMessage> for SMessageEdit {
  fn into(self) -> UpdateMessage {
    UpdateMessage {
      content: self.content,
      message_type: self.message_type,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SMessageStatus {
  NotSent,
  Sent,
  Seen,
}
impl Into<MessageStatus> for SMessageStatus {
  fn into(self) -> MessageStatus {
    match self {
      Self::NotSent => MessageStatus::NotSent,
      Self::Sent => MessageStatus::Sent,
      Self::Seen => MessageStatus::Seen,
    }
  }
}

impl From<MessageStatus> for SMessageStatus {
  fn from(value: MessageStatus) -> SMessageStatus {
    match value {
      MessageStatus::NotSent => Self::NotSent,
      MessageStatus::Sent => Self::Sent,
      MessageStatus::Seen => Self::Seen,
    }
  }
}
