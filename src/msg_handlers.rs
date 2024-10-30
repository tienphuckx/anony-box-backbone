use crate::database::models::NewMessageText;
use crate::database::schema::users;
use crate::database::schema::{groups, messages_text, participants};
use crate::errors::DBError;
use crate::payloads::common::CommonResponse;
use crate::payloads::messages::{GetMessagesRequest, GetMessagesResponse, MessageResponse};
use crate::payloads::messages::{MessageWithUser, SendMessageRequest, SendMessageResponse};
use crate::AppState;
use axum::extract::Path;
use axum::{extract::State, Json};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;

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
    .first::<(i32, i32)>(conn)
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
   Get list latest messages by group_id
   Query the latest 10 messages for the specified group with a join to include user information
*/
pub async fn get_latest_messages(
  State(app_state): State<Arc<AppState>>,
  Json(request): Json<GetMessagesRequest>,
) -> Result<Json<GetMessagesResponse>, DBError> {
  tracing::debug!("POST: /get-latest-messages by group id");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;
  let messages = messages_text::table
    .inner_join(users::table.on(users::id.eq(messages_text::user_id)))
    .filter(messages_text::group_id.eq(request.group_id))
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
      tracing::error!("Error querying messages with user info: {:?}", err);
      DBError::QueryError("Error querying messages".to_string())
    })?;

  // build response
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
