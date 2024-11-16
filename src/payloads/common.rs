use crate::{utils::minors::calculate_offset_from_page, DEFAULT_PAGE_SIZE, DEFAULT_PAGE_START};
use axum::{http::StatusCode, response::IntoResponse, Json};
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

#[derive(Deserialize, Debug, ToSchema)]
pub enum OrderBy {
  ASC,
  DESC,
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
impl PageRequest {
  pub fn get_offset_and_limit(&self) -> (u64, i64) {
    let page = self.get_page();
    let per_page = self.get_per_page() as i64;
    let offset = calculate_offset_from_page(page as u64, per_page as u64);
    (offset, per_page)
  }
  pub fn get_page(&self) -> u16 {
    let mut page = self.page.unwrap_or(DEFAULT_PAGE_START);
    if page == 0 {
      page = DEFAULT_PAGE_START;
    }
    page
  }
  pub fn get_per_page(&self) -> u32 {
    self.limit.unwrap_or(DEFAULT_PAGE_SIZE) as u32
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
impl<T> IntoResponse for ListResponse<T>
where
  T: Serialize,
{
  fn into_response(self) -> axum::response::Response {
    (StatusCode::OK, Json(self)).into_response()
  }
}
