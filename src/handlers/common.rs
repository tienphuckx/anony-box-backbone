use crate::{
  database::models::User, errors::ApiError, services::user::get_user_by_code, PoolPGConnectionType,
};

/// ### Handler for API "/"
#[utoipa::path(get, path = "/")]
pub async fn home() -> &'static str {
  tracing::debug!("GET :: /");
  "Let's quick chat with AnonymousChatBox"
}

pub async fn fallback() -> &'static str {
  "The requested URL was not found on the server."
}

pub async fn check_user_exists(
  conn: &mut PoolPGConnectionType,
  user_code: Option<String>,
) -> Result<User, ApiError> {
  if user_code.is_none() {
    return Err(ApiError::Forbidden);
  }
  let user = get_user_by_code(conn, &user_code.unwrap())
    .map_err(|_| ApiError::new_database_query_err("Failed to retrieve user by code"))?;
  if let Some(user) = user {
    return Ok(user);
  } else {
    return Err(ApiError::NotFound("User".into()));
  }
}
