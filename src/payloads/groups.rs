use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
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

#[derive(Serialize, Default, ToSchema)]
pub struct GroupResult {
  pub user_id: i32,
  pub username: String,
  pub user_code: String,
  pub group_id: i32,
  pub group_name: String,
  pub group_code: String,
  pub expired_at: String,
  pub is_waiting: bool,
}
#[derive(Deserialize)]
pub struct JoinGroupForm {
  pub group_code: String,
  pub username: String,
  pub message: String,
}

/**
 for api get list gr by user id
*/
#[derive(Serialize, ToSchema)]
pub struct GroupInfo {
  pub group_id: i32,
  pub group_name: String,
  pub group_code: String,
  pub expired_at: String,
  pub latest_ms_content: String,
  pub latest_ms_time: String,
  pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct GroupListResponse {
  pub user_id: i32,
  pub user_code: String,
  pub total_gr: usize,
  pub list_gr: Vec<GroupInfo>,
}

/**
  for create a group with user id and others field
  case: user already exists
*/
#[derive(Deserialize)]
pub struct NewGroupWithUserIdRequest {
  pub user_id: i32,
  pub group_name: String,
  pub duration: u32,
  pub maximum_members: Option<i32>,
  pub approval_require: Option<bool>,
}

#[derive(Serialize)]
pub struct GroupResponse {
  pub group_id: i32,
  pub group_name: String,
  pub group_code: String,
  pub expired_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct WaitingListResponse {
  pub id: i32,
  pub user_id: i32,
  pub username: String,
  pub message: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ProcessWaitingRequest {
  pub is_approved: bool,
}
