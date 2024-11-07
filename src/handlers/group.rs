use std::{borrow::Borrow, sync::Arc, time::Duration};
use diesel::result::Error;
use axum::{
  body::Body, extract::{Path, Query, State}, http::StatusCode, Json
};
use chrono::{Utc};
use diesel::{
  r2d2::ConnectionManager, result::DatabaseErrorKind, Connection, ExpressionMethods, JoinOnDsl,
  OptionalExtension, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use r2d2::PooledConnection;
use tracing::error;
use crate::{
  database::{
    models::{self, Group, NewGroup, NewWaitingList, User, WaitingList},
    schema::{self},
  }, errors::{ApiError, DBError}, extractors::UserToken, payloads::{
    self,
    common::{ListResponse, PageRequest},
    groups::{GroupResult, JoinGroupForm, NewGroupForm, ProcessWaitingRequest, WaitingListResponse},
  }, services::{
    self, group::{check_owner_of_group, check_user_join_group, get_count_waiting_list, get_waiting_list_object}, user::{create_user, get_user_by_code}
  }, utils::{
    crypto::generate_secret_code,
    minors::{calculate_offset_from_page, calculate_total_pages},
  }, AppState, DEFAULT_PAGE_SIZE, DEFAULT_PAGE_START
};

use crate::payloads::groups::{DelGroupRequest, DelGroupResponse, GrDetailSettingResponse, GroupInfo, GroupListResponse, LeaveGroupRequest, LeaveGroupResponse, NewUserAndGroupRequest, NewUserAndGroupResponse, RmUserRequest, RmUserResponse, UserSettingInfo};

use crate::database::schema::{attachments, groups, messages, messages_text, participants, users, waiting_list};

use crate::payloads::common::CommonResponse;

use crate::payloads::groups::{GroupResponse, NewGroupWithUserIdRequest};


/// ### Create new or get existing user from user_code token
///
/// This function will return a new or existing user depend on user's existence:
/// - If user_code doesn't provide or if having but not valid a new user will be created.
/// - If user existed in database return existing user.
fn get_or_create_user_from_user_code(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  user_code: &Option<String>,
  username: &str,
) -> Result<(User, bool), diesel::result::Error> {
  let user;
  let mut is_new = true;
  if user_code.is_none() {
    tracing::debug!("");
    user = create_user(conn, username)?;
  } else {
    let user_code = user_code.as_ref().unwrap();
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
/// 1. Checks if the user exists using the `user_code` token.
/// 2. If the user exists in the database, utilize the existing user; otherwise, create a new user.
/// 3. Create a new group.
/// 4. Add the current user to the participants table of the newly created group.
#[utoipa::path(
  post,
  path = "/add-user-group",
  params(
    (
      "x-user-code" = Option<String>, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
  ),
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
  UserToken(user_token): UserToken,
  Json(new_group_form): Json<NewGroupForm>,
) -> Result<Json<GroupResult>, DBError> {
  tracing::debug!("POST: /add-user-group");
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;
  let transaction_rs: Result<(User, Group), diesel::result::Error> = conn.transaction(|conn| {
    let (user, _) = get_or_create_user_from_user_code(conn, user_token.borrow(), &new_group_form.username)?;

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
  Ok(Json(group_rs))
}

pub async fn create_user_and_group_v1(
    State(app_state): State<Arc<AppState>>,
    UserToken(user_token): UserToken,
    Json(request): Json<NewUserAndGroupRequest>,
) -> Result<Json<CommonResponse<NewUserAndGroupResponse>>, ApiError> {
    tracing::debug!("POST: /v1/add-user-group");

    let conn = &mut app_state
        .db_pool
        .get()
        .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

    // Step 1: Check if the username already exists
    let existing_user = schema::users::table
        .filter(schema::users::username.eq(&request.username))
        .first::<User>(conn)
        .optional()
        .map_err(|err| {
            error!("Error checking if username exists: {:?}", err);
            ApiError::DatabaseError(DBError::QueryError("Failed to check username".to_string()))
        })?;

    // If the user exists, return an API error response with a structured message
    if existing_user.is_some() {
        return Err(ApiError::ExistedResource(format!(
            "Username '{}' is already taken",
            request.username
        )));
    }


    // Step 2: Begin transaction to create user and group
    let transaction_rs: Result<NewUserAndGroupResponse, Error> = conn.transaction(|conn| {
        // Retrieve or create the user
        let (user, _) = get_or_create_user_from_user_code(conn, user_token.borrow(), &request.username)?;

        // Calculate current and expiration times
        let current = Utc::now();
        let expired_at = current + Duration::from_secs((request.duration * 60) as u64);

        // Create a new group
        let new_group = NewGroup {
            name: &request.group_name,
            maximum_members: request.maximum_members,
            approval_require: request.approval_require,
            user_id: user.id,
            created_at: current.naive_local(),
            expired_at: expired_at.naive_local(),
            group_code: &generate_secret_code(&request.group_name),
        };

        let group_result = diesel::insert_into(schema::groups::table)
            .values(&new_group)
            .returning(Group::as_returning())
            .get_result::<Group>(conn)?;

        // Insert the user into the participants table as a participant of the new group
        diesel::insert_into(schema::participants::table)
            .values((
                schema::participants::user_id.eq(user.id),
                schema::participants::group_id.eq(group_result.id),
            ))
            .execute(conn)?;

        let group_rs = payloads::groups::GroupResult {
            user_id: user.id,
            username: user.username,
            user_code: user.user_code,
            group_id: group_result.id,
            group_name: group_result.name,
            group_code: group_result.group_code,
            expired_at: group_result.expired_at.unwrap().and_utc().to_string(),
            is_waiting: false,
        };

        // Construct the success response
        Ok(NewUserAndGroupResponse {
            msg: format!("User '{}' and group '{}' created successfully.", request.username, request.group_name),
            gr: group_rs
        })
    });

    // Map the result into a common JSON response format
    match transaction_rs {
        Ok(response) => Ok(Json(CommonResponse::success(response))),
        Err(err) => {
            error!("Transaction error: {:?}", err);
            Err(ApiError::DatabaseError(DBError::TransactionError(
                "Failed to create user and group".to_string(),
            )))
        }
    }
}

/// ### Handler for the `/join-group`
///
/// This handler manages user requests to join a group, with the following operations:
/// 1. **User Validation**:
///    - Checks for an existing `user_code` token to verify if the user exists.
///    - If the user exists in the database, uses the existing user data; otherwise, creates a new user entry.
///
/// 2. **Group Joining Process**:
///    - **Pending Approval**: If the group requires owner approval, the user is added to a waiting list.
///    - **Direct Join**: If no owner approval is required, the user is added to the group immediately.
#[utoipa::path(
  post,
  path = "/join-group",
  params(
    (
      "x-user-code" = Option<String>, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
  ),
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
  UserToken(user_token): UserToken,
  Json(join_group_form): Json<JoinGroupForm>,
) -> Result<Json<GroupResult>, ApiError> {
  tracing::debug!("POST: /join-group");
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
  let transaction_rs: Result<Result<(User, Group, bool), ApiError>, diesel::result::Error> = conn
    .transaction(|conn| {
      let (user, _) =
        get_or_create_user_from_user_code(conn, &user_token, &join_group_form.username)?;

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
        let waiting_list = NewWaitingList {
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

  Ok(Json(group_rs))
}

/// ### Handler for the `/gr/list/{user_id}`
///
/// This api return list group of user by user id, display in left bar (desktop)
/// 1. **User Validation**:
///    - Checks for an existing `user_id`
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
        groups::created_at,
      ))
      .load::<(i32, String, String, Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>)>(conn)
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
        .map(|(group_id, group_name, group_code, expired_at, created_at)| {
            tracing::info!(
                "Group found: group_id = {}, group_name = {}, group_code = {}, expired_at = {}, created_at = {}",
                group_id,
                group_name,
                group_code,
                expired_at.map_or("None".to_string(), |dt| dt.to_string()),
                created_at.map_or("None".to_string(), |dt| dt.to_string())
            );

            use diesel::dsl::sql;
            use diesel::sql_types::{Nullable, Text, Timestamp};

            let latest_message = messages_text::table
                .inner_join(users::table.on(messages_text::user_id.eq(users::id)))
                .filter(messages_text::group_id.eq(group_id))
                .order(messages_text::created_at.desc())
                .select((
                    sql::<Nullable<Text>>("messages_text.content"),
                    sql::<Timestamp>("messages_text.created_at"),
                    sql::<Nullable<Text>>("users.username"),
                ))
                .first::<(Option<String>, chrono::NaiveDateTime, Option<String>)>(conn)
                .optional()
                .map_err(|err| {
                    tracing::error!(
            "Failed to get latest message for group_id {}: {:?}", group_id, err
        );
                    DBError::QueryError(format!("Error loading latest message: {:?}", err))
                })?;

            let (latest_ms_content, latest_ms_time, latest_ms_username) = latest_message
                .map(|(content, time, username)| (
                    content.unwrap_or_default(),
                    time,
                    username.unwrap_or_default(),
                ))
                .unwrap_or_default();

        Ok(GroupInfo {
          group_id,
          group_name,
          group_code,
          expired_at: expired_at.unwrap_or_default().to_string(),
          latest_ms_content,
          latest_ms_time: latest_ms_time.to_string(),
          latest_ms_username,
          created_at: created_at.unwrap_or_default().to_string(),
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

///### Validate user is an owner of the group_id or not
///
/// If user is not an owner an api error will be propagated
fn validate_owner_of_group(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  user_token: &Option<String>,
  group_id: i32,
) -> Result<(), ApiError> {
  if user_token.is_none() {
    return Err(ApiError::Forbidden);
  }
  let current_user = get_user_by_code(conn, user_token.as_ref().unwrap())
    .map_err(|_| ApiError::new_database_query_err("Unable to get current user from database"))?;

  if current_user.is_none() {
    return Err(ApiError::Forbidden);
  }
  let User { id: user_id, .. } = current_user.unwrap();

  if !check_owner_of_group(conn, user_id, group_id)
    .map_err(|_| ApiError::new_database_query_err("Failed to check owner of group"))?
  {
    return Err(ApiError::Unauthorized);
  }
  Ok(())
}

/// ### Handler for API `/group/:group_id/waiting-list`
///
/// Get waiting list from specific group id
///
/// **Notice**: User must be an owner of the group
///

#[utoipa::path(
  get,
  path = "/groups/{group_id}/waiting-list",
  params(
    (
      "x-user-code" = String, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
    ("group_id" = u32, Path, description = "id of the group"),
    ("page" = Option<u32>, Query, description = "page index", ),
    ("limit" = Option<u32>, Query, description = "the number of items per a page")
  ),
  responses(
      (status = 200, description = "Get waiting list successfully",
      body = ListResponse<WaitingListResponse>, content_type = "application/json",
        example = json!(
            {
                "count": 2,
                "total_pages": 1,
                "objects": [
                  {
                    "id": 2,
                    "user_id": 39,
                    "username": "thanhnguyen",
                    "message": "Hello my join request 1"
                  },
                  {
                    "id": 4,
                    "user_id": 40,
                    "username": "sangtien",
                    "message": "Hello my join request 2"
                  }
                ]
              }
              
        )),
      (status = 404, description = "The group does not have any waiting request"),
      (status = 403, description = "The current user doesn't have permission to access the resource"),
      (status = 401, description = "The current user doesn't have right to access the resource"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn get_waiting_list(
  State(app_state): State<Arc<AppState>>,
  UserToken(user_token) : UserToken,
  Path(group_id): Path<i32>,
  Query(page): Query<PageRequest>,
) -> Result<(StatusCode, Json<ListResponse<WaitingListResponse>>), ApiError> {
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

  validate_owner_of_group(conn, &user_token, group_id)?;

  let PageRequest { page, limit } = page;
  let mut page = page.unwrap_or(DEFAULT_PAGE_START);
  if page == 0{
    page = DEFAULT_PAGE_START;
  }
  let per_page = limit.unwrap_or(DEFAULT_PAGE_SIZE) as i64;
  let offset = calculate_offset_from_page(page as u64, per_page as u64);
  use schema::waiting_list::dsl::group_id as w_group_id;

  let waiting_objects: Vec<(WaitingList, User)> = schema::waiting_list::table
    .inner_join(schema::users::table)
    .filter(w_group_id.eq(group_id))
    .limit(per_page)
    .offset(offset as i64)
    .select((WaitingList::as_select(), User::as_select()))
    .load::<(WaitingList, User)>(conn)
    .map_err(|_| {
      ApiError::DatabaseError(DBError::QueryError("Could not get waiting list".into()))
    })?;
  if waiting_objects.is_empty() {
    return Err(ApiError::NotFound(
      "No waiting list items".into()
    ));
  }
  let waiting_objects = waiting_objects
    .iter()
    .map(|object| WaitingListResponse {
      id: object.0.id,
      user_id: object.1.id,
      username: object.1.username.clone(),
      message: object.0.message.clone().unwrap_or_default(),
    })
    .collect::<Vec<WaitingListResponse>>();
  let count = get_count_waiting_list(conn, group_id).map_err(|_| {
    ApiError::DatabaseError(DBError::QueryError(
      "Could not get amount of waiting list".into(),
    ))
  })?;
  let total_pages = calculate_total_pages(count as u64, per_page as u64);
  tracing::debug!("total_pages: {}", total_pages);
  let response = ListResponse {
    count: count as i32,
    total_pages: total_pages as u16,
    objects: waiting_objects,
  };

  Ok((StatusCode::OK, Json(response)))
}

/// ### Handler for API `/waiting-list/:request_id`
///
/// Process joining request: accept or reject the request
///
/// **Notice**: User must be an owner of the group
///
#[utoipa::path(
  post,
  path = "/waiting-list/{request_id}",
  params(
    (
      "x-user-code" = String, Header, description = "user code for authentication",
      example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
    ),
    ("request_id" = u32, Path, description = "id of the request"),
  ),
  request_body = ProcessWaitingRequest,
  responses(
      (status = 200, description = "Processes waiting list item successfully"),
      (status = 404, description = "Not found joining request"),
      (status = 403, description = "The current user doesn't have permission to access the resource"),
      (status = 401, description = "The current user doesn't have right to access the resource"),
      (status = 500, description = "Database error")
  ),
)]
pub async fn process_joining_request(
  State(app_state): State<Arc<AppState>>,
  UserToken(user_token): UserToken,
  Path(request_id): Path<i32>,
  
  Json(process_form): Json<ProcessWaitingRequest>,
) -> Result<(StatusCode, Body), ApiError> {
  let conn = &mut app_state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

  let join_request = get_waiting_list_object(conn, request_id)
    .map_err(|_|ApiError::new_database_query_err("Unable to get waiting list"))?
    .ok_or(ApiError::NotFound("Not found joining request".into()))?;
  
  validate_owner_of_group(conn, &user_token, join_request.group_id)?;
  
  services::group::process_joining_request(conn, join_request, process_form.is_approved)
  .map_err(|_|ApiError::new_database_query_err("Unable to process joining request"))?;

  Ok((StatusCode::OK, Body::empty()))
}

#[utoipa::path(
    post,
    path = "/del-gr",
    request_body = DelGroupRequest,
    responses(
        (status = 200, description = "Group deleted successfully", body = CommonResponse<DelGroupResponse>),
        (status = 404, description = "User or group not found", body = CommonResponse<String>),
        (status = 401, description = "User not authorized to delete this group", body = CommonResponse<String>),
        (status = 500, description = "Database error", body = CommonResponse<String>)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn del_gr_req(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<DelGroupRequest>,
) -> Result<Json<CommonResponse<DelGroupResponse>>, ApiError> {
    let conn = &mut app_state
        .db_pool
        .get()
        .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

    // Check if the user exists
    let is_user_exists = users::table
        .find(req.u_id)
        .first::<models::User>(conn)
        .optional()
        .map_err(|err| {
            tracing::error!("Error checking user_id {}: {:?}", req.u_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking user".to_string()))
        })?;

    if is_user_exists.is_none() {
        return Ok(Json(CommonResponse::error(1, "User does not exist")));
    }

    // Check if the group exists and is not expired
    use schema::groups::dsl::{groups};
    let group = groups
        .find(req.gr_id)
        .select(Group::as_select())
        .first::<Group>(conn)
        .optional()
        .map_err(|err| {
            tracing::error!("Error checking group_id {}: {:?}", req.gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking group".to_string()))
        })?;


    if let Some(group) = group {
        // Check if the user is the owner of the group
        if !check_owner_of_group(conn, req.u_id, req.gr_id)
            .map_err(|_| ApiError::new_database_query_err("Failed to check owner of group"))?
        {
            return Err(ApiError::Unauthorized);
        }

        // Step 1: Delete attachments linked to messages in this group
        diesel::delete(attachments::table.filter(
            attachments::message_id.eq_any(
                messages::table
                    .select(messages::id)
                    .filter(messages::group_id.eq(req.gr_id))
            )
        ))
            .execute(conn)
            .map_err(|err| {
                tracing::error!("Failed to delete attachments for group_id {}: {:?}", req.gr_id, err);
                ApiError::DatabaseError(DBError::QueryError("Failed to delete attachments".to_string()))
            })?;

        // Step 2: Delete messages in the messages table for this group
        diesel::delete(messages::table.filter(messages::group_id.eq(req.gr_id)))
            .execute(conn)
            .map_err(|err| {
                tracing::error!("Failed to delete messages for group_id {}: {:?}", req.gr_id, err);
                ApiError::DatabaseError(DBError::QueryError("Failed to delete messages".to_string()))
            })?;

        // Step 3: Delete messages in the messages_text table for this group
        diesel::delete(messages_text::table.filter(messages_text::group_id.eq(req.gr_id)))
            .execute(conn)
            .map_err(|err| {
                tracing::error!("Failed to delete messages_text for group_id {}: {:?}", req.gr_id, err);
                ApiError::DatabaseError(DBError::QueryError("Failed to delete messages_text".to_string()))
            })?;

        // Step 4: Delete participants related to this group
        diesel::delete(participants::table.filter(participants::group_id.eq(req.gr_id)))
            .execute(conn)
            .map_err(|err| {
                tracing::error!("Failed to delete participants for group_id {}: {:?}", req.gr_id, err);
                ApiError::DatabaseError(DBError::QueryError("Failed to delete participants".to_string()))
            })?;

        // Step 5: Delete waiting_list entries related to this group
        diesel::delete(waiting_list::table.filter(waiting_list::group_id.eq(req.gr_id)))
            .execute(conn)
            .map_err(|err| {
                tracing::error!("Failed to delete waiting_list for group_id {}: {:?}", req.gr_id, err);
                ApiError::DatabaseError(DBError::QueryError("Failed to delete waiting_list entries".to_string()))
            })?;

        // Step 6: Finally, delete the group itself
        diesel::delete(groups.find(req.gr_id))
            .execute(conn)
            .map_err(|err| {
                tracing::error!("Failed to delete group_id {}: {:?}", req.gr_id, err);
                ApiError::DatabaseError(DBError::QueryError("Failed to delete group".to_string()))
            })?;


        // Return successful deletion response
        let response = DelGroupResponse {
            gr_id: group.id,
            gr_code: group.group_code,
            del_status: "Deleted successfully".to_string(),
        };

        Ok(Json(CommonResponse::success(response)))
    } else {
        Ok(Json(CommonResponse::error(1, "Group does not exist or is expired")))
    }
}

#[utoipa::path(
    get,
    path = "/group-detail/setting/{gr_id}/{u_id}",
    params(
    ("gr_id" = i32, Path, description = "id of the group"),
     ("u_id" = i32, Path, description = "id of user"),
    ),
    responses(
        (status = 200, description = "Get Group Detail Setting successfully", body = CommonResponse<GrDetailSettingResponse>),
        (status = 404, description = "User or group not found", body = CommonResponse<String>),
        (status = 401, description = "User not authorized to delete this group", body = CommonResponse<String>),
        (status = 500, description = "Database error", body = CommonResponse<String>)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn get_gr_setting(
    State(app_state): State<Arc<AppState>>,
    Path((gr_id, user_id)): Path<(i32, i32)>,
) -> Result<Json<CommonResponse<GrDetailSettingResponse>>, ApiError> {
    let conn = &mut app_state
        .db_pool
        .get()
        .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

    // Check if the user exists
    let is_user_exists = users::table
        .find(user_id)
        .first::<models::User>(conn)
        .optional()
        .map_err(|err| {
            tracing::error!("Error checking user_id {}: {:?}", user_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking user".to_string()))
        })?;

    if is_user_exists.is_none() {
        return Ok(Json(CommonResponse::error(1, "User does not exist")));
    }

    // Check if the group exists and is not expired
    use schema::groups::dsl::{groups};
    let group = groups
        .find(gr_id)
        .select(Group::as_select())
        .first::<Group>(conn)
        .optional()
        .map_err(|err| {
            tracing::error!("Error checking group_id {}: {:?}", gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking group".to_string()))
        })?;


    if let Some(group) = group {

        // check user owner of the gr
        if !check_owner_of_group(conn, user_id, gr_id)
            .map_err(|_| ApiError::new_database_query_err("Failed to check owner of group"))?
        {
            return Err(ApiError::Unauthorized);
        }

        let total_joined_member = participants::table
            .filter(participants::group_id.eq(gr_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|err| {
                tracing::error!("Error counting joined members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to count joined members".to_string()))
            })? as i32;

        // Query to get list of joined members
        let list_joined_member: Vec<UserSettingInfo> = participants::table
            .inner_join(users::table.on(users::id.eq(participants::user_id)))
            .filter(participants::group_id.eq(gr_id))
            .select((users::id, users::username, users::user_code))
            .load::<(i32, String, String)>(conn)
            .map_err(|err| {
                tracing::error!("Error fetching joined members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to fetch joined members".to_string()))
            })?
            .into_iter()
            .map(|(user_id, username, user_code)| UserSettingInfo {
                user_id,
                username,
                user_code,
            })
            .collect();

        // Query to count total waiting members
        let total_waiting_member = waiting_list::table
            .filter(waiting_list::group_id.eq(gr_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|err| {
                tracing::error!("Error counting waiting members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to count waiting members".to_string()))
            })? as i32;

        // Query to get list of waiting members
        let list_waiting_member: Vec<UserSettingInfo> = waiting_list::table
            .inner_join(users::table.on(users::id.eq(waiting_list::user_id)))
            .filter(waiting_list::group_id.eq(gr_id))
            .select((users::id, users::username, users::user_code))
            .load::<(i32, String, String)>(conn)
            .map_err(|err| {
                tracing::error!("Error fetching waiting members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to fetch waiting members".to_string()))
            })?
            .into_iter()
            .map(|(user_id, username, user_code)| UserSettingInfo {
                user_id,
                username,
                user_code,
            })
            .collect();

        let response = GrDetailSettingResponse {
            group_id: group.id,
            owner_id: group.user_id,
            group_name: group.name,
            group_code: group.group_code,
            expired_at: group.expired_at.map_or("N/A".to_string(), |ts| ts.to_string()),
            created_at: group.created_at.map_or("N/A".to_string(), |ts| ts.to_string()),
            maximum_members: group.maximum_members.unwrap_or_default(),
            total_joined_member,
            list_joined_member,
            total_waiting_member,
            list_waiting_member,
        };

        Ok(Json(CommonResponse::success(response)))

    } else {
        Ok(Json(CommonResponse::error(1, "Group does not exist or is expired")))
    }
}

#[utoipa::path(
    get,
    path = "/group-detail/setting/{gr_id}",
    params(
    ("gr_id" = i32, Path, description = "id of the group"),
    ),
    responses(
        (status = 200, description = "Get Group Detail Setting successfully", body = CommonResponse<GrDetailSettingResponse>),
        (status = 404, description = "User or group not found", body = CommonResponse<String>),
        (status = 401, description = "User not authorized to delete this group", body = CommonResponse<String>),
        (status = 500, description = "Database error", body = CommonResponse<String>)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn get_gr_setting_v1(
    State(app_state): State<Arc<AppState>>,
    Path(gr_id): Path<i32>,
) -> Result<Json<CommonResponse<GrDetailSettingResponse>>, ApiError> {
    let conn = &mut app_state
        .db_pool
        .get()
        .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

    use schema::groups::dsl::{groups};
    let group = groups
        .find(gr_id)
        .select(Group::as_select())
        .first::<Group>(conn)
        .optional()
        .map_err(|err| {
            tracing::error!("Error checking group_id {}: {:?}", gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking group".to_string()))
        })?;


    if let Some(group) = group {

        let total_joined_member = participants::table
            .filter(participants::group_id.eq(gr_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|err| {
                tracing::error!("Error counting joined members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to count joined members".to_string()))
            })? as i32;

        // Query to get list of joined members
        let list_joined_member: Vec<UserSettingInfo> = participants::table
            .inner_join(users::table.on(users::id.eq(participants::user_id)))
            .filter(participants::group_id.eq(gr_id))
            .select((users::id, users::username, users::user_code))
            .load::<(i32, String, String)>(conn)
            .map_err(|err| {
                tracing::error!("Error fetching joined members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to fetch joined members".to_string()))
            })?
            .into_iter()
            .map(|(user_id, username, user_code)| UserSettingInfo {
                user_id,
                username,
                user_code,
            })
            .collect();

        // Query to count total waiting members
        let total_waiting_member = waiting_list::table
            .filter(waiting_list::group_id.eq(gr_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|err| {
                tracing::error!("Error counting waiting members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to count waiting members".to_string()))
            })? as i32;

        // Query to get list of waiting members
        let list_waiting_member: Vec<UserSettingInfo> = waiting_list::table
            .inner_join(users::table.on(users::id.eq(waiting_list::user_id)))
            .filter(waiting_list::group_id.eq(gr_id))
            .select((users::id, users::username, users::user_code))
            .load::<(i32, String, String)>(conn)
            .map_err(|err| {
                tracing::error!("Error fetching waiting members: {:?}", err);
                ApiError::DatabaseError(DBError::QueryError("Failed to fetch waiting members".to_string()))
            })?
            .into_iter()
            .map(|(user_id, username, user_code)| UserSettingInfo {
                user_id,
                username,
                user_code,
            })
            .collect();

        let response = GrDetailSettingResponse {
            group_id: group.id,
            owner_id: group.user_id,
            group_name: group.name,
            group_code: group.group_code,
            expired_at: group.expired_at.map_or("N/A".to_string(), |ts| ts.to_string()),
            created_at: group.created_at.map_or("N/A".to_string(), |ts| ts.to_string()),
            maximum_members: group.maximum_members.unwrap_or_default(),
            total_joined_member,
            list_joined_member,
            total_waiting_member,
            list_waiting_member,
        };

        Ok(Json(CommonResponse::success(response)))

    } else {
        Ok(Json(CommonResponse::error(1, "Group does not exist or is expired")))
    }
}

#[utoipa::path(
    post,
    path = "/rm-u-from-gr",
    request_body = RmUserRequest,
    responses(
        (status = 200, description = "Group deleted successfully", body = RmUserResponse),
        (status = 404, description = "User or group not found", body = RmUserResponse),
        (status = 401, description = "User not authorized to delete this group", body = RmUserResponse),
        (status = 500, description = "Database error", body = RmUserResponse)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn rm_user_from_gr(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<RmUserRequest>,
) -> Result<Json<RmUserResponse>, ApiError> {
    tracing::debug!("POST: /rm-user-from-group");

    // Get a database connection from the pool
    let conn = &mut app_state
        .db_pool
        .get()
        .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

    // Check if the group exists
    use schema::groups::dsl::{groups};
    let group = groups
        .find(req.gr_id)
        .select(Group::as_select()) // Explicitly selecting the fields
        .first::<Group>(conn)
        .optional()
        .map_err(|err| {
            tracing::debug!("Error checking group_id {}: {:?}", req.gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking group existence".to_string()))
        })?;

    // Return error if group does not exist
    if group.is_none() {
        return Err(ApiError::NotFound("Group not found".to_string()));
    }

    // Check if the requesting user is the group owner
    if !check_owner_of_group(conn, req.gr_owner_id, req.gr_id)
        .map_err(|_| ApiError::DatabaseError(DBError::QueryError("Failed to verify group ownership".to_string())))?
    {
        return Err(ApiError::Unauthorized);
    }

    use schema::participants::dsl::{participants, user_id, group_id};
    let delete_result = diesel::delete(participants.filter(user_id.eq(req.rm_user_id)).filter(group_id.eq(req.gr_id)))
        .execute(conn)
        .map_err(|err| {
            tracing::debug!("Error removing user {} from group {}: {:?}", req.rm_user_id, req.gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error removing user from group".to_string()))
        })?;

    // If no rows were deleted, the user was not part of the group
    if delete_result == 0 {
        return Err(ApiError::NotFound("User not found in the specified group".to_string()));
    }

    // Return success response
    Ok(Json(RmUserResponse {
        res_code: 200,
        res_msg: "User successfully removed from the group".to_string(),
    }))
}


#[utoipa::path(
    post,
    path = "/leave-gr",
    request_body = LeaveGroupRequest,
    responses(
        (status = 200, description = "Group deleted successfully", body = LeaveGroupResponse),
        (status = 404, description = "User or group not found", body = LeaveGroupResponse),
        (status = 500, description = "Database error", body = LeaveGroupResponse)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn user_leave_gr(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<LeaveGroupRequest>,
) -> Result<Json<LeaveGroupResponse>, ApiError> {
    tracing::debug!("POST: /leave-gr");

    // Get a database connection from the pool
    let conn = &mut app_state
        .db_pool
        .get()
        .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;

    // Check if the group exists
    use schema::groups::dsl::{groups};
    let group = groups
        .find(req.gr_id)
        .select(Group::as_select()) // Explicitly selecting the fields
        .first::<Group>(conn)
        .optional()
        .map_err(|err| {
            tracing::debug!("Error checking group_id {}: {:?}", req.gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error checking group existence".to_string()))
        })?;

    // Return error if group does not exist
    if group.is_none() {
        return Err(ApiError::NotFound("Group not found".to_string()));
    }

    use schema::participants::dsl::{participants, user_id, group_id};
    let delete_result = diesel::delete(participants.filter(user_id.eq(req.u_id)).filter(group_id.eq(req.gr_id)))
        .execute(conn)
        .map_err(|err| {
            tracing::debug!("Error removing user {} from group {}: {:?}", req.u_id, req.gr_id, err);
            ApiError::DatabaseError(DBError::QueryError("Error removing user from group".to_string()))
        })?;

    // If no rows were deleted, the user was not part of the group
    if delete_result == 0 {
        return Err(ApiError::NotFound("User not found in the specified group".to_string()));
    }

    // Return success response
    Ok(Json(LeaveGroupResponse {
        code: 200,
        msg: "User successfully leaved from the group".to_string(),
    }))
}