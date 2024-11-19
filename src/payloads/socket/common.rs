use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ResultMessage {
  pub status_code: i32,
  pub message: String,
}
impl ResultMessage {
  pub fn new(status_code: i32, message: &str) -> Self {
    Self {
      status_code,
      message: message.into(),
    }
  }
}
