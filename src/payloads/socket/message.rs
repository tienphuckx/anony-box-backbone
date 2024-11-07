use crate::{
  database::models::{Message, NewMessage},
  utils::minors::custom_serde::*,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SMessageType {
  Send(SNewMessage),
  Receive(SMessageContent),
  Edit(SMessageContent),
  Delete(Vec<i32>),
}

#[derive(Serialize, Clone, Deserialize, Debug, PartialEq)]

pub struct SMessageContent {
  pub user_id: i32,
  pub group_id: i32,
  pub content: String,
  #[serde(
    serialize_with = "serialize_date_time_utc",
    deserialize_with = "deserialize_with_utc"
  )]
  pub created_at: DateTime<Utc>,
  pub status: SMessageStatus,
}
impl From<Message> for SMessageContent {
  fn from(value: Message) -> Self {
    Self {
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
  pub content: String,

  #[serde(
    serialize_with = "serialize_date_time_utc",
    deserialize_with = "deserialize_with_utc"
  )]
  pub created_at: DateTime<Utc>,
}
impl<'a> SNewMessage {
  pub fn build_new_message(&'a self, user_id: i32, group_id: i32) -> NewMessage<'a> {
    NewMessage {
      user_id,
      group_id,
      content: Some(&self.content),
      created_at: self.created_at.naive_utc(),
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
