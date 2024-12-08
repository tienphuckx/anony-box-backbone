use crate::database::models::{ MessageStatus, MessageTypeEnum, NewMessage};
use crate::errors::{ApiError, DBError};
use crate::extractors::UserToken;
use crate::payloads::common::{ListResponse, PageRequest, OrderBy};
use crate::payloads::messages::{ AttachmentPayload, MessageFilterParams, MessageResponse, MessageSortParams, MessageWithUser, UpdateMessage};
use crate::payloads::messages::{SendMessageRequest, SendMessageResponse};
use crate::utils::minors::calculate_total_pages;
use crate::{services, AppState};
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;

use super::common::check_user_exists;

/// ### Handler for API POST `/messages`
///
/// This handler performs the following tasks:
/// 1. Checks if the user exists using the `user_code` token.
/// 2. If the user exists in the database
/// 3. Checks if user has joined the group
/// 4. Create a new message in database
#[utoipa::path(
  post,
  path = "/messages",
  params(
    (
      "x-user-code" = Option<String>, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
  ),
  request_body(
    description = "New message json",
    content(
        (SendMessageRequest = "application/json", example = json!(
          {
            "content": "This is new message",
            "group_id" : 32,
            "message_type": "TEXT",
            "message_uuid": "ff0e32e2-ab5e-4ef7-8dec-93668270ab8c",
            "attachments": [
              {
                "url": "http://127.0.0.1:8080/files/readme.md",
                "attachment_type": "TEXT"
              }
            ]
          }
        )),
    )
  ),
  responses(
      (status = 200, description = "Send a message successfully", body = SendMessageResponse, content_type = "application/json"),
      (status = 401, description = "The current user doesn't have right to access the resource"),
      (status = 404, description = "User not found"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn send_msg(
  State(app_state): State<Arc<AppState>>,
  UserToken(user_token): UserToken,
  Json(msg_request): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, ApiError> {
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
  let user = check_user_exists(conn, user_token).await?;

  if !services::group::check_user_join_group(conn, user.id, msg_request.group_id)
    .map_err(|_err| ApiError::new_database_query_err("Failed to check user joined group"))?
  {
    return Err(ApiError::Unauthorized);
  }

  // Insert the text message into `messages`
  let new_message = NewMessage {
    message_uuid: msg_request.message_uuid,
    content: msg_request.content.as_ref(), // Convert String to &str
    message_type: msg_request.message_type,
    status: MessageStatus::Sent,
    created_at: Utc::now().naive_utc(),
    user_id: user.id,
    group_id: msg_request.group_id,
  };

  let inserted_message = services::message::create_new_message(conn, new_message)
    .map_err(|_| ApiError::new_database_query_err("Failed to insert new message"))?;
  let message_id = inserted_message.id;
  let mut response = SendMessageResponse::from(inserted_message);
  // Insert attachment if the message payload has attachments
  if let Some(attachments) = msg_request.attachments {
    let new_attachments = attachments.iter()
    .map(|e|AttachmentPayload::into_new(e, message_id)).collect();
    let inserted_attachments = services::attachment::create_attachments(conn, new_attachments).map_err(ApiError::DatabaseError)?;
    response.set_attachment(inserted_attachments.iter().map(|e| AttachmentPayload::from(e.clone())).collect());
  }
  // Prepare the response
  Ok(Json(response))
}

/// ### Handler for GET /groups/:group_id/messages
#[utoipa::path(
  get,
  path = "/groups/{group_id}/messages",
  params(
    (
      "x-user-code" = String, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
    ("group_id" = u32, Path, description = "id of the group"),
    ("message_type" = Option<MessageTypeEnum>,Query, description = "message type enum filter"),
    ("content" = Option<String>, Query,description = "content text filter"),
    ("status" = Option<MessageStatus>, Query,description = "message status filter"),
    ("from_date" = Option<String>, Query, description = "from created date filter"),
    ("to_date" = Option<String>, Query, description = "to created date filter"),
    ("created_at_sort" = Option<OrderBy>, Query, description = "created at sort by ASC or DESC"),
    ("page" = Option<u32>, Query, description = "page index" ),
    ("limit" = Option<u32>, Query, description = "the number of items per a page")
  ),
  responses(
      (status = 200, description = "Get waiting list successfully",
      body = ListResponse<MessageWithUser>, content_type = "application/json",
        example = json!(
            {
                "count": 3,
                "total_pages": 12,
                "objects": [
                  {
                    "message_uuid": "16b7bedb-92c4-4888-a2fc-b01b5776e897",
                    "id": 1,
                    "content": "This is test message 1",
                    "message_type": "TEXT",
                    "attachments": [],
                    "status": "Sent",
                    "created_at": "2012-12-12 12:12:12",
                    "user_id": 44,
                    "user_name": "Linus Torvalds"
                  },
                  {
                    "message_uuid": "bf0e32e2-ab5e-4ef7-8dec-93668270ab8c",
                    "id": 2,
                    "content": "This is new message 2",
                    "message_type": "ATTACHMENT",
                    "attachments": [
                      {
                        "id": 2,
                        "url": "http://127.0.0.1:8080/files/readme.md",
                        "attachment_type": "TEXT"
                      },
                      {
                        "id": 3,
                        "url": "http://127.0.0.1:8080/files/avatar.png",
                        "attachment_type": "IMAGE"
                      }
                    ],
                    "status": "Sent",
                    "created_at": "2024-12-08T07:34:57.120623+00:00",
                    "updated_at": null,
                    "user_id": 2,
                    "user_name": "tienphuc"
                  },
                  {
                    "message_uuid": "ff0e32e2-ab5e-4ef7-8dec-93668270ab8c",
                    "id": 3,
                    "content": "This is update message 3",
                    "message_type": "TEXT",
                    "attachments": [],
                    "status": "Sent",
                    "created_at": "2024-11-16T06:51:52.784529+00:00",
                    "updated_at": "2024-11-16T06:59:47.420978+00:00",
                    "user_id": 1,
                    "user_name": "linhnguyen"
                  },
                ]
              }
              
        )),
      (status = 403, description = "The current user doesn't have permission to access the resource"),
      (status = 401, description = "The current user doesn't have right to access the resource"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn get_messages(
  State(app_state): State<Arc<AppState>>,
  Path(group_id): Path<i32>,
  UserToken(user_token): UserToken,
  Query(message_filters): Query<MessageFilterParams>,
  Query(page_request): Query<PageRequest>,
  Query(message_sorts): Query<MessageSortParams>,
) -> Result<ListResponse<MessageWithUser>, ApiError> {
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
  let user = check_user_exists(conn, user_token).await?;

  if !services::group::check_user_join_group(conn, user.id, group_id)
    .map_err(|_err| ApiError::new_database_query_err("Failed to check user joined group"))?
  {
    return Err(ApiError::Unauthorized);
  }
  // Query the latest messages using group_code
  let messages =
    services::message::get_messages(conn, group_id, &page_request, &message_filters, message_sorts)
      .map_err(ApiError::DatabaseError)?;
  
  let message_count = services::message::get_count_messages(conn, group_id, message_filters).map_err(ApiError::DatabaseError)?;
  let total_pages = calculate_total_pages(message_count as u64, page_request.get_per_page() as u64) as u16;
  let list_response = ListResponse {
    count: messages.len() as i32,
    objects: messages,
    total_pages,
  };
  Ok(list_response)
}


/// ### Handler for DELETE /messages/:message_id
#[utoipa::path(
  delete,
  path = "/messages/{message_id}",
  params(
    (
      "x-user-code" = String, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
    ("message_id" = u32, Path, description = "id of the group"),
  ),
  responses(
      (status = 204, description = "Delete message successfully"),
      (status = 403, description = "The current user doesn't have permission to access the resource"),
      (status = 401, description = "The current user doesn't have right to access the resource"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn delete_message(
  State(app_state): State<Arc<AppState>>,
  Path(message_id): Path<i32>,
  UserToken(user_token): UserToken,
) -> Result<(StatusCode,Body), ApiError> {
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
  let user = check_user_exists(conn, user_token).await?;
  
 let message = services::message::get_message(conn, message_id).map_err(ApiError::DatabaseError)?;

  if message.is_none(){
    return Err(ApiError::NotFound("Message".into()));
  }

  if message.unwrap().user_id != user.id{
    return Err(ApiError::Unauthorized);
  }

  // Query the latest messages using group_code
  let _  = services::message::delete_message(conn, message_id)
      .map_err(ApiError::DatabaseError)?;
  Ok((StatusCode::NO_CONTENT, Body::empty()))

  
}

/// ### Handler for PUT /messages/:message_id
#[utoipa::path(
  put,
  path = "/messages/{message_id}",
  params(
    (
      "x-user-code" = String, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
    ("message_id" = u32, Path, description = "id of the group"),
  ),
  request_body(
    description = "Update message json",
    content(
        (UpdateMessage = "application/json", example = json!(
          {
            "content": "This is new message",
            "message_type": "TEXT",
          }
        )),
    )
  ),
  responses(
      (status = 200, description = "Update the message successfully", body = MessageResponse, content_type = "application/json"),
      (status = 403, description = "The current user doesn't have permission to access the resource"),
      (status = 401, description = "The current user doesn't have right to access the resource"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn update_message(
  State(app_state): State<Arc<AppState>>,
  Path(message_id): Path<i32>,
  UserToken(user_token): UserToken,
  Json(update_data): Json<UpdateMessage>,
) -> Result<Json<MessageResponse>, ApiError> {
  let conn = &mut app_state
  .db_pool
  .get()
  .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
let user = check_user_exists(conn, user_token).await?;

let message = services::message::get_message(conn, message_id).map_err(ApiError::DatabaseError)?;
if message.is_none(){
  return Err(ApiError::NotFound("Message".into()));
}

if message.unwrap().user_id != user.id{
  return Err(ApiError::Unauthorized);
}

  let message = services::message::update_message(conn, message_id, update_data)
  .map_err(ApiError::DatabaseError)?;
  Ok(Json(MessageResponse::from(message)))
}