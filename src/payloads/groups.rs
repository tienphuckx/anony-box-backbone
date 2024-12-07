use crate::payloads::messages::MessageWithUser;
use crate::utils::custom_serde::*;
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

#[derive(Serialize, Deserialize, Default, ToSchema)]
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
  pub latest_ms_username: String,
  pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct GroupListResponse {
  pub user_id: i32,
  pub user_code: String,
  pub total_gr: usize,
  pub list_gr: Vec<GroupInfo>,
  pub list_waiting_gr: Vec<GroupInfo>,
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
  #[serde(serialize_with = "serialize_with_date_time_utc")]
  pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct ProcessWaitingRequest {
  pub is_approved: bool,
}

/// for api delete group
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DelGroupRequest {
  pub u_id: i32,
  pub gr_id: i32,
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
  pub gr_id: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LeaveGroupResponse {
  pub code: i32,
  pub msg: String,
}

/// Api get group detail setting

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GrDetailSettingResponse {
  pub group_id: i32,
  pub owner_id: i32,
  pub group_name: String,
  pub group_code: String,
  pub expired_at: String,
  pub created_at: String,
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

/// Api: create user and group at one time
#[derive(Serialize, Deserialize, ToSchema)]
pub struct NewUserAndGroupRequest {
  pub username: String,
  pub group_name: String,
  pub duration: u32,
  pub maximum_members: Option<i32>,
  pub approval_require: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NewUserAndGroupResponse {
  pub msg: String,
  pub gr: GroupResult,
}
#[derive(Serialize, ToSchema)]
pub struct GroupDetailResponse {
  pub group_name: String,
  pub user_id: i32,
  pub max_member: i32,
  pub joined_member: i32,
  pub waiting_member: i32,
  pub created_at: String,
  pub expired_at: String,
  pub messages: Vec<MessageWithUser>,
}

/// Api: remove an user from a griup
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RmUserRequest {
  pub gr_owner_id: i32,
  pub gr_id: i32,
  pub rm_user_id: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RmUserResponse {
  pub res_code: i32,
  pub res_msg: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RmRfGroupsRequest {
  pub cmd: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RmRfGroupsResponse {
  pub msg: String,
}
