use chrono::Utc;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper};

use crate::{
  database::{
    models::{self, User},
    schema::{self},
  },
  utils::crypto::generate_secret_code,
  PoolPGConnectionType,
};

pub fn create_user(
  conn: &mut PoolPGConnectionType,
  username: &str,
) -> Result<User, diesel::result::Error> {
  let new_user = models::NewUser {
    username,
    created_at: Utc::now().naive_local(),
    user_code: &&generate_secret_code(username),
  };

  let user_result = diesel::insert_into(schema::users::table)
    .values(&new_user)
    .returning(models::User::as_returning())
    .get_result::<models::User>(conn)?;
  Ok(user_result)
}

#[allow(dead_code)]
pub fn user_exists(
  conn: &mut PoolPGConnectionType,
  secret_code: &str,
) -> Result<bool, diesel::result::Error> {
  let count = schema::users::table
    .filter(schema::users::user_code.eq(secret_code))
    .count()
    .get_result::<i64>(conn)?;
  if count > 0 {
    Ok(true)
  } else {
    Ok(false)
  }
}

pub fn get_user_by_code(
  conn: &mut PoolPGConnectionType,
  secret_code: &str,
) -> Result<Option<User>, diesel::result::Error> {
  schema::users::table
    .filter(schema::users::user_code.eq(secret_code))
    .select(User::as_select())
    .first(conn)
    .optional()
}

pub fn get_user_ids_from_group(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
) -> Result<Vec<i32>, diesel::result::Error> {
  use schema::participants;
  let user_ids = participants::table
    .filter(participants::group_id.eq(group_id))
    .select(participants::user_id)
    .get_results::<i32>(conn)?;
  Ok(user_ids)
}
