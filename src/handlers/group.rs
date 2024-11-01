use std::{sync::Arc, time::Duration};

use axum::{
  extract::{Path, State},
  http::StatusCode,
  Json,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use chrono::Utc;
use diesel::{
  r2d2::ConnectionManager, result::DatabaseErrorKind, Connection, ExpressionMethods, JoinOnDsl,
  OptionalExtension, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use r2d2::PooledConnection;
use time::OffsetDateTime;

use crate::{
  database::{
    models::{self, Group, NewGroup, User, WaitingList},
    schema,
  },
  errors::{ApiError, DBError},
  payloads::{
    self,
    groups::{GroupResult, JoinGroupForm, NewGroupForm},
  },
  services::{
    group::check_user_join_group,
    user::{create_user, get_user_by_code},
  },
  utils::crypto::generate_secret_code,
  AppState,
};

use crate::payloads::groups::{GroupInfo, GroupListResponse};

use crate::database::schema::{groups, messages_text, participants, users};

use crate::payloads::common::CommonResponse;

use crate::payloads::groups::{GroupResponse, NewGroupWithUserIdRequest};

/// ### Create new or get existing user from user_code cookie
///
/// This function will return a new or existing user depend on user's existence:
/// - If user_cookie doesn't provide or if having but not valid a new user will be created.
/// - If user existed in database return existing user.
fn get_or_create_user_from_user_code(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  cookie_jar: &CookieJar,
  username: &str,
) -> Result<(User, bool), diesel::result::Error> {
  let user;
  let mut is_new = true;
  let user_code_cookie = cookie_jar.get("user_code");
  if user_code_cookie.is_none() {
    tracing::debug!("Not found user code");
    user = create_user(conn, username)?;
  } else {
    let user_code = user_code_cookie.unwrap().value();
    tracing::debug!("user_code: {}", user_code);
    if let Some(found_user) = get_user_by_code(conn, user_code)? {
      tracing::debug!("Found user from database via user_code");
      user = found_user;
      is_new = false;
    } else {
      user = create_user(conn, username)?;
    }
  }
  Ok((user, is_new))
}

/// ### Handler for API `/add-user-group`
///
/// This handler performs the following tasks:
/// 1. Checks if the user exists using the `user_code` cookie.
/// 2. If the user exists in the database, utilize the existing user; otherwise, create a new user.
/// 3. Create a new group.
/// 4. Add the current user to the participants table of the newly created group.
#[utoipa::path(
  post,
  path = "/add-user-group",
  request_body(
    description = "New group form ",
    content(
        (NewGroupForm = "application/json", example = json!(
          {
            "username": "LinhNguyen",
            "group_name": "Linux fundamentals",
            "duration": 60,
            "maximum_members": 50,
            "approval_require":  true
          }
        )),
    )
 ),
  responses(
      (status = 200, description = "Create a group successfully", body = GroupResult, content_type = "application/json"),
      (status = 400, description = "Username already existed"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn create_user_and_group(
  State(app_state): State<Arc<AppState>>,
  cookie_jar: CookieJar,
  Json(new_group_form): Json<NewGroupForm>,
) -> Result<(CookieJar, Json<GroupResult>), DBError> {
  tracing::debug!("POST: /add-user-group");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;
  let transaction_rs: Result<(User, Group), diesel::result::Error> = conn.transaction(|conn| {
    let (user, _) = get_or_create_user_from_user_code(conn, &cookie_jar, &new_group_form.username)?;

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
    is_waiting: false,
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

/// ### Handler for the `/join-group`
///
/// This handler manages user requests to join a group, with the following operations:
/// 1. **User Validation**:
///    - Checks for an existing `user_code` cookie to verify if the user exists.
///    - If the user exists in the database, uses the existing user data; otherwise, creates a new user entry.
///
/// 2. **Group Joining Process**:
///    - **Pending Approval**: If the group requires owner approval, the user is added to a waiting list.
///    - **Direct Join**: If no owner approval is required, the user is added to the group immediately.
#[utoipa::path(
  post,
  path = "/join-group",
  request_body(
    description = "Join group form ",
    content(
        (NewGroupForm = "application/json", example = json!(
          {
            "group_code": "5C28DBCFAB2EA1DD8EF3C1B2B363475F84A0A3031803798D1A3507F813548B6F",
            "username": "phucnguyen",
            "message": "Hello I want to join a group, please help me approve my request"
          }
        )),
    )
 ),
  responses(
      (status = 200, description = "Join group successfully", body = GroupResult, content_type = "application/json"),
      (status = 400, description = "User already join the group"),
      (status = 401, description = "User was already in waiting list"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn join_group(
  State(app_state): State<Arc<AppState>>,
  cookie_jar: CookieJar,
  Json(join_group_form): Json<JoinGroupForm>,
) -> Result<(CookieJar, Json<GroupResult>), ApiError> {
  tracing::debug!("POST: /join-group");
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
  let transaction_rs: Result<Result<(User, Group, bool), ApiError>, diesel::result::Error> = conn
    .transaction(|conn| {
      let (user, _) =
        get_or_create_user_from_user_code(conn, &cookie_jar, &join_group_form.username)?;

      use schema::groups::dsl::{group_code, groups};
      let group = groups
        .filter(group_code.eq(&join_group_form.group_code))
        .select(models::Group::as_select())
        .get_result::<models::Group>(conn)
        .optional()?;
      if group.is_none() {
        return Ok(Err(ApiError::NotFound(format!(
          "Not found group with user_code: {}",
          join_group_form.group_code,
        ))));
      }
      let group = group.unwrap();

      // checking user already joined the group
      if check_user_join_group(conn, user.id, group.id)? {
        return Ok(Err(ApiError::AlreadyJoined));
      }
      // check group approval_require property to consider add directly to group or waiting list
      let mut is_waiting = false;

      if group.approval_require.unwrap() {
        let waiting_list = WaitingList {
          user_id: user.id,
          group_id: group.id,
          message: Some(join_group_form.message.clone()),
          created_at: Utc::now().naive_utc(),
        };
        let insert_result = diesel::insert_into(schema::waiting_list::table)
          .values(waiting_list)
          .execute(conn);
        if let Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) =
          insert_result
        {
          return Ok(Err(ApiError::ExistedResource(
            "User was already in waiting list".into(),
          )));
        }
        is_waiting = true;
      } else {
        let insert_result = diesel::insert_into(schema::participants::table)
          .values((
            schema::participants::user_id.eq(user.id),
            schema::participants::group_id.eq(group.id),
          ))
          .execute(conn);
        if let Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) =
          insert_result
        {
          return Ok(Err(ApiError::AlreadyJoined));
        }
      }
      Ok(Ok((user, group, is_waiting)))
    });
  if let Ok(Err(err)) = transaction_rs {
    tracing::error!("API error: {}", err.to_string());
    return Err(err);
  }
  if let Err(err) = transaction_rs {
    tracing::error!("DB error: {}", err.to_string());
    return Err(ApiError::DatabaseError(DBError::QueryError(
      err.to_string(),
    )));
  }
  let (user, group, is_waiting) = transaction_rs.unwrap().unwrap();

  let group_rs = payloads::groups::GroupResult {
    user_id: user.id,
    username: user.username,
    user_code: user.user_code,
    group_id: group.id,
    group_name: group.name,
    group_code: group.group_code,
    expired_at: group.expired_at.unwrap().and_utc().to_string(),
    is_waiting,
  };

  let mut user_code_cookie = Cookie::new("user_code", group_rs.user_code.clone());
  user_code_cookie.set_http_only(true);
  let expired =
    OffsetDateTime::from_unix_timestamp(group.expired_at.unwrap().and_utc().timestamp()).unwrap();
  user_code_cookie.set_expires(expired);

  let new_jar = cookie_jar.add(user_code_cookie);
  Ok((new_jar, Json(group_rs)))
}

// Get user groups by user ID
#[utoipa::path(
  get,
  path = "/gr/list/{user_id}",
  params(
        ("user_id" = i32, Path, description = "ID of the user to get groups for")
  ),
  responses(
        (status = 200, description = "List of groups the user belongs to", body = GroupListResponse),
        (status = 404, description = "User not found", body = String),
        (status = 500, description = "Database connection error", body = String)
  )
)]
pub async fn get_list_groups_by_user_id(
  State(app_state): State<Arc<AppState>>,
  Path(user_id): Path<i32>,
) -> Result<(StatusCode, Json<GroupListResponse>), DBError> {
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

  // For each group, find the latest message
  let group_list: Result<Vec<GroupInfo>, DBError> = user_groups
      .into_iter()
      .map(|(group_id, group_name, group_code, expired_at)| {
        tracing::info!(
                "Group found: group_id = {}, group_name = {}, group_code = {}",
                group_id,
                group_name,
                group_code
            );

        // Get the latest message for this group
        let latest_message = messages_text::table
            .filter(messages_text::group_id.eq(group_id))
            .order(messages_text::created_at.desc())
            .select((
              messages_text::content,
              messages_text::created_at,
            ))
            .first::<(Option<String>, chrono::NaiveDateTime)>(conn)
            .optional()
            .map_err(|err| {
              tracing::error!(
                        "Failed to get latest message for group_id {}: {:?}", group_id, err
                    );
              DBError::QueryError(format!("Error loading latest message: {:?}", err))
            })?;

        let (latest_ms_content, latest_ms_time) = latest_message
            .map(|(content, time)| (content.unwrap_or_default(), time))
            .unwrap_or_default();

        Ok(GroupInfo {
          group_id,
          group_name,
          group_code,
          expired_at: expired_at.unwrap_or_default().to_string(),
          latest_ms_content,
          latest_ms_time: latest_ms_time.to_string(),
        })
      })
      .collect();

  let group_list = group_list?;

  tracing::info!("Total groups for user_id {}: {}", user_id, group_list.len());

  let response = GroupListResponse {
    user_id: user.id,
    user_code: user.user_code,
    total_gr: group_list.len(),
    list_gr: group_list,
  };

  Ok((StatusCode::OK, Json(response)))
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
