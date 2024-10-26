use axum::{extract::State, Json};
use crate::database::schema::{messages_text, participants};
use crate::errors::DBError;
use crate::payloads::common::CommonResponse;
use crate::payloads::messages::{SendMessageRequest, SendMessageResponse};
use crate::AppState;
use diesel::prelude::*;
use std::sync::Arc;
use chrono::Utc;
use crate::database::models::NewMessageText;

pub async fn send_msg(
    State(app_state): State<Arc<AppState>>,
    Json(msg_req): Json<SendMessageRequest>,
) -> Result<Json<CommonResponse<SendMessageResponse>>, DBError> {
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
        return Ok(Json(CommonResponse::error(1, "User is not part of the group")));
    }

    // Insert the text message into `messages_text`
    let new_message = NewMessageText {
        content: Some(msg_req.content.as_str()),  // Convert String to &str
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
