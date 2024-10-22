use std::sync::Arc;
use std::time::Duration;

use axum::response::Result as AxumResult;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use diesel::{Connection, RunQueryDsl, SelectableHelper};

use crate::database::models::NewGroup;
use crate::database::{models, schema};
use crate::errors::DBError;
use crate::utils::crypto::generate_secret_code;
use crate::{
  payloads::groups::{GroupResult, NewGroupForm},
  AppState,
};

pub async fn hello() -> &'static str {
  "Hello this is anonymous home page"
}

pub async fn create_group(
  State(app_state): State<Arc<AppState>>,
  Json(new_group_form): Json<NewGroupForm>,
) -> AxumResult<(StatusCode, Json<GroupResult>)> {
  let conn = &mut app_state.db_pool.get().map_err(DBError::ConnectionError)?;

  let transaction_rs: Result<GroupResult, diesel::result::Error> = conn.transaction(|conn| {
    let new_user = models::NewUser {
      username: &new_group_form.username,
      created_at: Utc::now().naive_local(),
      user_code: &generate_secret_code(&new_group_form.username),
    };

    let user_result = diesel::insert_into(schema::users::table)
      .values(&new_user)
      .returning(models::User::as_returning())
      .get_result::<models::User>(conn)?;
    let current = Utc::now();
    let expired_at = current + Duration::from_secs((new_group_form.duration * 60) as u64);

    let new_group = NewGroup {
      name: &new_group_form.group_name,
      maximum_members: new_group_form.maximum_members,
      approval_require: new_group_form.approval_require,
      user_id: user_result.id,
      created_at: current.naive_local(),
      expired_at: expired_at.naive_local(),
      group_code: &generate_secret_code(&new_group_form.group_name),
    };

    let group_result = diesel::insert_into(schema::groups::table)
      .values(&new_group)
      .returning(models::Group::as_returning())
      .get_result::<models::Group>(conn)?;
    let group_rs = GroupResult {
      username: user_result.username,
      user_code: user_result.user_code,
      group_name: group_result.name,
      group_code: group_result.group_code,
      expired_at: group_result.expired_at.unwrap().and_utc().to_string(),
    };

    Ok(group_rs)
  });
  let group_rs = transaction_rs.map_err(|err| match err {
    diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
      DBError::ConstraintViolation(err.to_string())
    }
    _ => DBError::QueryError(err.to_string()),
  })?;
  Ok((StatusCode::OK, Json(group_rs)))
}
