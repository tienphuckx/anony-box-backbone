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


/// for api delete group
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DelGroupRequest {
  pub u_id: i32,
  pub gr_id: i32
}

#[derive(Serialize, ToSchema)]
pub struct DelGroupResponse {
  pub gr_id: i32,
  pub gr_code: String,
  pub del_status: String,
}

/// for api leave group
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LeaveGroupRequest {
  pub u_id: i32,
  pub gr_id: i32
}

#[derive(Serialize, ToSchema)]
pub struct LeaveGroupResponse {
  pub gr_id: i32,
  pub gr_code: String,

}

/// Api get group detail setting

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GrDetailSettingResponse {
  pub group_id: i32,
  pub group_name: String,
  pub group_code: String,
  pub expired_at: String,
  pub maximum_members: i32,
  pub total_joined_member: i32,
  pub list_joined_member: Vec<UserSettingInfo>,
  pub total_waiting_member: i32,
  pub list_waiting_member: Vec<UserSettingInfo>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserSettingInfo {
  pub user_id: i32,
  pub username: String,
  pub user_code: String,
}
