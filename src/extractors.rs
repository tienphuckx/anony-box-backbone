use axum::{
  async_trait,
  extract::FromRequestParts,
  http::{request::Parts, StatusCode},
};

pub struct UserToken(pub Option<String>);

#[async_trait]
impl<S> FromRequestParts<S> for UserToken
where
  S: Send + Sync,
{
  type Rejection = (StatusCode, &'static str);

  async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
    if let Some(authorization_value) = parts.headers.get("x-user-code") {
      tracing::debug!("x-user-code header: {:?}", authorization_value);
      if !authorization_value.is_empty() {
        if let Ok(token) = authorization_value.to_str() {
          if token.is_empty() {
            return Err((
              StatusCode::BAD_REQUEST,
              "Authorization token must be provided",
            ));
          }
          return Ok(UserToken(Some(token.to_string())));
        }
      }
    }
    Ok(UserToken(None))
  }
}
