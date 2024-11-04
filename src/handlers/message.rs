use crate::database::models::NewMessageText;
use crate::database::schema::{users, waiting_list};
use crate::database::schema::{groups, messages_text, participants};
use crate::errors::DBError;
use crate::payloads::common::CommonResponse;
use crate::payloads::messages::{GetMessagesResponse, GroupDetailResponse, MessageResponse};
use crate::payloads::messages::{MessageWithUser, SendMessageRequest, SendMessageResponse};
use crate::AppState;
use axum::extract::Path;
use axum::{extract::State, Json};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use diesel::dsl::count;

pub async fn send_msg(
  State(app_state): State<Arc<AppState>>,
  Json(msg_req): Json<SendMessageRequest>,
) -> Result<Json<CommonResponse<SendMessageResponse>>, DBError> {
  tracing::debug!("POST: /send-msg");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;

  // Check if the user is part of the group
  let participant_exists = participants::table
    .filter(participants::user_id.eq(msg_req.user_id))
    .filter(participants::group_id.eq(msg_req.group_id))
    .first::<(i32, i32, i32)>(conn)
    .optional()
    .map_err(|err| {
      tracing::error!(
        "Error checking participant for user_id {} and group_id {}: {:?}",
        msg_req.user_id,
        msg_req.group_id,
        err
      );
      DBError::QueryError("Error checking participant".to_string())
    })?;

  if participant_exists.is_none() {
    return Ok(Json(CommonResponse::error(
      1,
      "User is not part of the group",
    )));
  }

  // Insert the text message into `messages_text`
  let new_message = NewMessageText {
    content: Some(msg_req.content.as_str()), // Convert String to &str
    message_type: msg_req.message_type.as_str(),
    created_at: Utc::now().naive_utc(),
    user_id: msg_req.user_id,
    group_id: msg_req.group_id,
  };

  let message_id = diesel::insert_into(messages_text::table)
    .values(&new_message)
    .returning(messages_text::id)
    .get_result::<i32>(conn)
    .map_err(|err| {
      tracing::error!("Error inserting message: {:?}", err);
      DBError::QueryError("Error inserting message".to_string())
    })?;

  // Prepare the response
  let response = SendMessageResponse {
    message_id,
    content: msg_req.content.clone(),
    message_type: msg_req.message_type.clone(),
    created_at: Utc::now().to_rfc3339(),
  };

  Ok(Json(CommonResponse::success(response)))
}

/*
   Get group message detail by group id
   Query the latest 10 messages for the specified group with a join to include user information
*/
/// ### Handler for the `/get_group_detail_by_group_id`
///
/// This api select group detail message detail by group id
/// 1. **List messages**:
///    - default is 10 message (TODO)
///
/// 2. **Each message item**:
///    - Message content
///    - username
///    - time
///    - reaction data
pub async fn get_group_detail_with_extra_info(
    State(app_state): State<Arc<AppState>>,
    Path(group_id): Path<i32>,
) -> Result<Json<GroupDetailResponse>, DBError> {
    tracing::debug!("GET: /group/detail/{}", group_id);

    let conn = &mut app_state.db_pool.get().map_err(|err| {
        tracing::error!("Failed to get connection from pool: {:?}", err);
        DBError::ConnectionError(err)
    })?;

    let group_info = groups::table
        .filter(groups::id.eq(group_id))
        .select((
            groups::name,
            groups::created_at.nullable(),
            groups::expired_at.nullable(),
            groups::maximum_members.nullable(),
        ))
        .first::<(String, Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>, Option<i32>)>(conn)
        .optional()
        .map_err(|err| {
            tracing::error!("Failed to get group info for group_id {}: {:?}", group_id, err);
            DBError::QueryError(format!("Group not found: {:?}", err))
        })?;

    // Check if group_info is None, return an error if no group is found
    let (group_name, created_at, expired_at, max_member) = match group_info {
        Some(info) => info,
        None => return Err(DBError::QueryError("Group not found".to_string())),
    };

    // Count joined members
    let joined_member = participants::table
        .filter(participants::group_id.eq(group_id))
        .select(count(participants::user_id))
        .first::<i64>(conn)
        .map_err(|err| {
            tracing::error!("Failed to count joined members for group_id {}: {:?}", group_id, err);
            DBError::QueryError(format!("Error counting joined members: {:?}", err))
        })?;

    // Count waiting members
    let waiting_member = waiting_list::table
        .filter(waiting_list::group_id.eq(group_id))
        .select(count(waiting_list::user_id))
        .first::<i64>(conn)
        .map_err(|err| {
            tracing::error!("Failed to count waiting members for group_id {}: {:?}", group_id, err);
            DBError::QueryError(format!("Error counting waiting members: {:?}", err))
        })?;

    // Fetch messages (limit to latest messages if needed)
    let messages = messages_text::table
        .filter(messages_text::group_id.eq(group_id))
        .inner_join(users::table.on(users::id.eq(messages_text::user_id)))
        .order(messages_text::created_at.asc())
        .select((
            messages_text::id,
            messages_text::content,
            messages_text::message_type,
            messages_text::created_at,
            messages_text::user_id,
            users::username,
        ))
        .load::<MessageWithUser>(conn)
        .map_err(|err| {
            tracing::error!("Failed to load messages for group_id {}: {:?}", group_id, err);
            DBError::QueryError(format!("Error loading messages: {:?}", err))
        })?;

    // Build response with max_member included
    let response = GroupDetailResponse {
        group_name,
        max_member: max_member.unwrap_or_default(), // Use default if max_member is None
        joined_member: joined_member as i32,
        waiting_member: waiting_member as i32,
        created_at: created_at.map(|dt| dt.to_string()).unwrap_or_default(),
        expired_at: expired_at.map(|dt| dt.to_string()).unwrap_or_default(),
        messages,
    };

    Ok(Json(response))
}



pub async fn get_latest_messages_by_code(
  State(app_state): State<Arc<AppState>>,
  Path(group_code): Path<String>,
) -> Result<Json<GetMessagesResponse>, DBError> {
  tracing::debug!("GET: /get-latest-messages/{}", group_code);
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;

  // Query the latest messages using group_code
  let messages = messages_text::table
    .inner_join(users::table.on(users::id.eq(messages_text::user_id)))
    .inner_join(groups::table.on(groups::id.eq(messages_text::group_id)))
    .filter(groups::group_code.eq(group_code))
    .order(messages_text::created_at.desc())
    .limit(10)
    .select((
      messages_text::id,
      messages_text::content,
      messages_text::message_type,
      messages_text::created_at,
      messages_text::user_id,
      users::username,
    ))
    .load::<MessageWithUser>(conn)
    .map_err(|err| {
      tracing::error!(
        "Error querying messages with user info by group code: {:?}",
        err
      );
      DBError::QueryError("Error querying messages".to_string())
    })?;

  // Build the response
  let messages_response = messages
    .into_iter()
    .map(|message| MessageResponse {
      id: message.id,
      content: message.content,
      message_type: message.message_type,
      created_at: message.created_at,
      user_id: message.user_id,
      user_name: message.user_name,
    })
    .collect();

  Ok(Json(GetMessagesResponse {
    messages: messages_response,
  }))
}
