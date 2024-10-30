use std::sync::Arc;

use crate::database::models;
use crate::database::schema::users;
use crate::errors::DBError;
use crate::payloads::common::CommonResponse;
use crate::payloads::user::{NewUserRequest, UserResponse};
use crate::utils::crypto::generate_secret_code;
use crate::AppState;
use axum::{extract::State, Json};

use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper};

/// Add User
#[utoipa::path(
    post,
    path = "/add-user",
    request_body = NewUserRequest,
    responses(
        (status = 200, description = "User successfully added", body = CommonResponse<UserResponse>),
        (status = 400, description = "Username already exists", body = CommonResponse<String>)
    )
)]
pub async fn add_user_docs(
  State(app_state): State<Arc<AppState>>,
  Json(new_user_req): Json<NewUserRequest>,
) -> Result<Json<CommonResponse<UserResponse>>, DBError> {
  tracing::debug!("POST: /add-user");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;

  // Check if the username already exists
  let existing_user = users::table
    .filter(users::username.eq(&new_user_req.username))
    .first::<models::User>(conn)
    .optional()
    .map_err(|err| {
      tracing::error!("Error checking username: {:?}", err);
      DBError::QueryError("Error checking username".to_string())
    })?;

  if let Some(_user) = existing_user {
    return Ok(Json(CommonResponse::error(1, "Username already exists")));
  }

  // Create a new user
  let new_user = models::NewUser {
    username: &new_user_req.username,
    created_at: chrono::Utc::now().naive_local(),
    user_code: &generate_secret_code(&new_user_req.username),
  };

  let inserted_user = diesel::insert_into(users::table)
    .values(&new_user)
    .returning(models::User::as_returning())
    .get_result::<models::User>(conn)
    .map_err(|err| {
      tracing::error!("Error inserting user: {:?}", err);
      DBError::QueryError("Error inserting user".to_string())
    })?;

  // Prepare the response
  let user_response = UserResponse {
    user_id: inserted_user.id,
    username: inserted_user.username,
    user_code: inserted_user.user_code,
  };

  Ok(Json(CommonResponse::success(user_response)))
}

/**
   Add a new user
*/
pub async fn add_user(
  State(app_state): State<Arc<AppState>>,
  Json(new_user_req): Json<NewUserRequest>,
) -> Result<Json<CommonResponse<UserResponse>>, DBError> {
  tracing::debug!("POST: /add-user");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;

  // Check if the username already exists
  let existing_user = users::table
    .filter(users::username.eq(&new_user_req.username))
    .first::<models::User>(conn)
    .optional()
    .map_err(|err| {
      tracing::error!("Error checking username: {:?}", err);
      DBError::QueryError("Error checking username".to_string())
    })?;

  if let Some(_user) = existing_user {
    return Ok(Json(CommonResponse::error(1, "Username already exists")));
  }

  // Create a new user
  let new_user = models::NewUser {
    username: &new_user_req.username,
    created_at: chrono::Utc::now().naive_local(),
    user_code: &generate_secret_code(&new_user_req.username),
  };

  let inserted_user = diesel::insert_into(users::table)
    .values(&new_user)
    .returning(models::User::as_returning())
    .get_result::<models::User>(conn)
    .map_err(|err| {
      tracing::error!("Error inserting user: {:?}", err);
      DBError::QueryError("Error inserting user".to_string())
    })?;

  // Prepare the response
  let user_response = UserResponse {
    user_id: inserted_user.id,
    username: inserted_user.username,
    user_code: inserted_user.user_code,
  };

  Ok(Json(CommonResponse::success(user_response)))
}
