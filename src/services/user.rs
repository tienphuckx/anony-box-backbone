use chrono::Utc;
use diesel::{
  r2d2::ConnectionManager, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl,
  RunQueryDsl, SelectableHelper,
};
use r2d2::PooledConnection;

use crate::{
  database::{
    models::{self, User},
    schema::{self},
  },
  utils::crypto::generate_secret_code,
};

pub fn create_user(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  username: &str,
) -> Result<User, diesel::result::Error> {
  let new_user = models::NewUser {
    username: username,
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
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
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
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  secret_code: &str,
) -> Result<Option<User>, diesel::result::Error> {
  schema::users::table
    .filter(schema::users::user_code.eq(secret_code))
    .select(User::as_select())
    .first(conn)
    .optional()
}
