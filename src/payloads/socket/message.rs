use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SMessageType {
  Send(SMessageContent),
  Receive(SMessageContent),
  Edit(SMessageContent),
  Delete(Vec<i32>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]

pub struct SMessageContent {
  pub user_id: i32,
  pub group_id: i32,
  pub content: String,
  pub status: SMessageStatus,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SMessageStatus {
  Sent,
  InProgress,
  Error,
}
