use std::sync::Arc;
use std::time::Duration;

use axum::{extract::State, Json};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use diesel::{Connection, RunQueryDsl, SelectableHelper};

use time::OffsetDateTime;

use crate::database::models::{Group, NewGroup, User};
use crate::database::{models, schema};
use crate::errors::DBError;

use crate::services::user::{create_user, get_user_by_code};
use crate::utils::crypto::generate_secret_code;
use crate::{
  payloads,
  payloads::groups::{GroupResult, NewGroupForm},
  AppState,
};

use crate::database::schema::{groups, participants, users};
use crate::payloads::groups::{GroupInfo, GroupListResponse};
use axum::extract::Path;
use diesel::prelude::*;

use crate::payloads::common::CommonResponse;
use crate::payloads::user::{NewUserRequest, UserResponse};

use crate::payloads::groups::{GroupResponse, NewGroupWithUserIdRequest};

pub async fn home() -> &'static str {
  tracing::debug!("GET :: /");
  "Let's quick chat with NosBox"
}

/// ### Handler for API `/add-user-group`
///
/// This handler performs the following tasks:
/// 1. Checks if the user exists using the `user_code` cookie.
/// 2. If the user exists in the database, utilize the existing user; otherwise, create a new user.
/// 3. Create a new group.
/// 4. Add the current user to the participants table of the newly created group.
pub async fn create_user_and_group(
  State(app_state): State<Arc<AppState>>,
  cookie_jar: CookieJar,
  Json(new_group_form): Json<NewGroupForm>,
) -> Result<(CookieJar, Json<GroupResult>), DBError> {
  tracing::debug!("POST: /add-user-group");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;
  let transaction_rs: Result<(User, Group), diesel::result::Error> = conn.transaction(|conn| {
    let user;
    let user_code_cookie = cookie_jar.get("user_code");
    if user_code_cookie.is_none() {
      tracing::debug!("Not found user code");
      user = create_user(conn, &new_group_form.username)?;
    } else {
      let user_code = user_code_cookie.unwrap().value();
      tracing::debug!("user_code: {}", user_code);
      if let Some(found_user) = get_user_by_code(conn, user_code)? {
        tracing::debug!("Found user from database via user_code");
        user = found_user;
      } else {
        user = create_user(conn, &new_group_form.username)?;
      }
    }

    let current = Utc::now();
    let expired_at = current + Duration::from_secs((new_group_form.duration * 60) as u64);

    let new_group = NewGroup {
      name: &new_group_form.group_name,
      maximum_members: new_group_form.maximum_members,
      approval_require: new_group_form.approval_require,
      user_id: user.id,
      created_at: current.naive_local(),
      expired_at: expired_at.naive_local(),
      group_code: &generate_secret_code(&new_group_form.group_name),
    };

    let group_result = diesel::insert_into(schema::groups::table)
      .values(&new_group)
      .returning(models::Group::as_returning())
      .get_result::<models::Group>(conn)?;

    // Insert the user into the participants table as a participant of the new group
    diesel::insert_into(schema::participants::table)
      .values((
        schema::participants::user_id.eq(user.id),
        schema::participants::group_id.eq(group_result.id),
      ))
      .execute(conn)?;

    Ok((user, group_result))
  });

  let (user, group) = transaction_rs.map_err(|err| match err {
    diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
      DBError::ConstraintViolation(err.to_string())
    }
    _ => DBError::QueryError(err.to_string()),
  })?;

  let group_rs = payloads::groups::GroupResult {
    user_id: user.id,
    username: user.username,
    user_code: user.user_code,
    group_id: group.id,
    group_name: group.name,
    group_code: group.group_code,
    expired_at: group.expired_at.unwrap().and_utc().to_string(),
  };
  // Add user code cookie to response with expired_at time of newly created group
  let mut user_code_cookie = Cookie::new("user_code", group_rs.user_code.clone());
  user_code_cookie.set_http_only(true);
  let expired =
    OffsetDateTime::from_unix_timestamp(group.expired_at.unwrap().and_utc().timestamp()).unwrap();
  user_code_cookie.set_expires(expired);

  let new_jar = cookie_jar.add(user_code_cookie);
  Ok((new_jar, Json(group_rs)))
}

/**
   Get list group that created or joined by current user
   Param: user_id
*/
pub async fn get_user_groups(
  State(app_state): State<Arc<AppState>>,
  Path(user_id): Path<i32>,
) -> Result<Json<GroupListResponse>, DBError> {
  tracing::debug!("GET: /gr/list/{}", user_id);
  let conn = &mut app_state.db_pool.get().map_err(|err| {
    tracing::error!("Failed to get connection from pool: {:?}", err);
    DBError::ConnectionError(err)
  })?;

  // Fetch user info
  let user = users::table
    .find(user_id)
    .first::<models::User>(conn)
    .map_err(|err| {
      tracing::error!("Failed to find user with id {}: {:?}", user_id, err);
      DBError::QueryError(format!("User not found: {:?}", err))
    })?;

  tracing::info!(
    "User found: user_id = {}, user_code = {}",
    user.id,
    user.user_code
  );

  // Fetch groups that the user is part of
  let user_groups = participants::table
    .inner_join(groups::table.on(groups::id.eq(participants::group_id)))
    .filter(participants::user_id.eq(user_id))
    .select((
      groups::id,
      groups::name,
      groups::group_code,
      groups::expired_at,
    ))
    .load::<(i32, String, String, Option<chrono::NaiveDateTime>)>(conn)
    .map_err(|err| {
      tracing::error!("Failed to load groups for user_id {}: {:?}", user_id, err);
      DBError::QueryError(format!("Error loading groups: {:?}", err))
    })?;

  tracing::info!(
    "Groups found for user_id {}: {}",
    user_id,
    user_groups.len()
  );

  if user_groups.is_empty() {
    tracing::warn!("No groups found for user_id {}", user_id);
  }

  let group_list: Vec<GroupInfo> = user_groups
    .into_iter()
    .map(|(group_id, group_name, group_code, expired_at)| {
      tracing::info!(
        "Group found: group_id = {}, group_name = {}, group_code = {}",
        group_id,
        group_name,
        group_code
      );
      GroupInfo {
        group_id,
        group_name,
        group_code,
        expired_at: expired_at.unwrap().and_utc().to_string(),
      }
    })
    .collect();
  tracing::info!("Total groups for user_id {}: {}", user_id, group_list.len());
  let response = GroupListResponse {
    user_id: user.id,
    user_code: user.user_code,
    total_gr: group_list.len(),
    list_gr: group_list,
  };
  Ok(Json(response))
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

/**
   Create a new group with exists user by user_id
*/
pub async fn create_group_with_user(
  State(app_state): State<Arc<AppState>>,
  Json(new_group_req): Json<NewGroupWithUserIdRequest>,
) -> Result<Json<CommonResponse<GroupResponse>>, DBError> {
  tracing::debug!("POST: /create-group");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;

  // Check if the user exists
  let user_exists = users::table
    .find(new_group_req.user_id)
    .first::<models::User>(conn)
    .optional()
    .map_err(|err| {
      tracing::error!(
        "Error checking user_id {}: {:?}",
        new_group_req.user_id,
        err
      );
      DBError::QueryError("Error checking user".to_string())
    })?;

  if user_exists.is_none() {
    return Ok(Json(CommonResponse::error(1, "User does not exist")));
  }

  let current_time = Utc::now();
  let expired_at = current_time + chrono::Duration::minutes(new_group_req.duration.into());

  // Create the new group
  let new_group = models::NewGroup {
    name: &new_group_req.group_name,
    group_code: &generate_secret_code(&new_group_req.group_name),
    user_id: new_group_req.user_id,
    approval_require: new_group_req.approval_require,
    created_at: current_time.naive_utc(),
    expired_at: expired_at.naive_utc(),
    maximum_members: new_group_req.maximum_members,
  };

  let group_result = diesel::insert_into(groups::table)
    .values(&new_group)
    .returning(models::Group::as_returning())
    .get_result::<models::Group>(conn)
    .map_err(|err| {
      tracing::error!("Error inserting group: {:?}", err);
      DBError::QueryError("Error inserting group".to_string())
    })?;

  // Insert into participants table
  diesel::insert_into(participants::table)
    .values((
      participants::user_id.eq(new_group_req.user_id),
      participants::group_id.eq(group_result.id),
    ))
    .execute(conn)
    .map_err(|err| {
      tracing::error!("Error inserting into participants: {:?}", err);
      DBError::QueryError("Error inserting into participants".to_string())
    })?;

  // Prepare the response
  let group_response = GroupResponse {
    group_id: group_result.id,
    group_name: group_result.name,
    group_code: group_result.group_code,
    expired_at: group_result.expired_at.unwrap().and_utc().to_string(),
  };

  Ok(Json(CommonResponse::success(group_response)))
}