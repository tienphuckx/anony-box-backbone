use crate::DEFAULT_PAGE_SIZE;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct CommonResponse<T> {
  pub code: i32,
  pub msg: String,
  pub data: Option<T>,
}

impl<T> CommonResponse<T> {
  pub fn success(data: T) -> Self {
    CommonResponse {
      code: 0,
      msg: "Success".to_string(),
      data: Some(data),
    }
  }

  pub fn error(code: i32, msg: &str) -> Self {
    CommonResponse {
      code,
      msg: msg.to_string(),
      data: None,
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct PageRequest {
  pub page: Option<u16>,
  pub limit: Option<u32>,
}
impl Default for PageRequest {
  fn default() -> Self {
    Self {
      page: Some(0),
      limit: Some(DEFAULT_PAGE_SIZE),
    }
  }
}

#[derive(Serialize, ToSchema, Debug)]
pub struct ListResponse<T> {
  pub count: i32,
  pub total_pages: u16,
  pub objects: Vec<T>,
}

impl<T> Default for ListResponse<T> {
  fn default() -> Self {
    Self {
      count: 0,
      total_pages: 0,
      objects: Vec::new(),
    }
  }
}
