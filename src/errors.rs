use axum::{http::StatusCode, response::IntoResponse};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBError {
  #[error("Failed to query from database {}", 0)]
  QueryError(String),
  #[error("Failed to get a connection: {0}")]
  ConnectionError(#[from] r2d2::Error),
  #[error("Constraint violation: {0}")]
  ConstraintViolation(String),
}

impl IntoResponse for DBError {
  fn into_response(self) -> axum::response::Response {
    match self {
      Self::QueryError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
      DBError::ConstraintViolation(err) => {
        (StatusCode::BAD_REQUEST, err.to_string()).into_response()
      }
      Self::ConnectionError(err) => {
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
      }
    }
  }
}

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum ApiError {
  #[error("The resource is not found: {0}")]
  NotFound(String),

  #[error("The {0} already existed")]
  ExistedResource(String),

  #[error("Unknown error")]
  Unknown,
}
