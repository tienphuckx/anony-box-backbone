use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct NewGroupForm {
  pub username: String,
  pub group_name: String,
  pub duration: u32,
  pub maximum_members: Option<i32>,
  pub approval_require: Option<bool>,
}
#[allow(dead_code)]
impl NewGroupForm {
  pub fn get_expired_time(&self) -> DateTime<Utc> {
    let now = Utc::now();
    now + Duration::minutes(self.duration as i64)
  }
}

#[derive(Serialize, Default)]
pub struct GroupResult {
  pub username: String,
  pub user_code: String,
  pub group_name: String,
  pub group_code: String,
  pub expired_at: String,
}
