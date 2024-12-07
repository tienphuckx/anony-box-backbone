use std::{env, path::PathBuf};

use axum_extra::extract::CookieJar;
use chrono::Utc;

use crate::{DEFAULT_SERVER_ADDRESS, DEFAULT_SERVER_PORT};

#[allow(dead_code)]
pub fn get_value_from_cookie(cookie_jar: CookieJar, key: &str) -> Option<String> {
  let cookie_value = cookie_jar.get(key);
  if cookie_value.is_none() {
    return None;
  }
  let value = cookie_value.unwrap().value();
  if value.is_empty() {
    return None;
  }
  return Some(value.to_string());
}

pub fn calculate_total_pages(count: u64, per_page: u64) -> u64 {
  if count % per_page > 0 {
    count / per_page + 1
  } else {
    count / per_page
  }
}

pub fn calculate_offset_from_page(page: u64, per_page: u64) -> u64 {
  if page == 0 {
    1
  } else {
    (page - 1) * per_page
  }
}

pub fn generate_file_name_with_timestamp(file_name: &str) -> String {
  let mut rs = String::new();
  let timestamp = Utc::now().timestamp();
  rs.push_str(&timestamp.to_string());
  rs.push_str("_");
  rs.push_str(file_name);
  rs
}

pub fn get_server_url() -> String {
  let server_addr = env::var("SERVER_ADDRESS").unwrap_or(DEFAULT_SERVER_ADDRESS.to_string());
  let server_port = if let Ok(value) = env::var("SERVER_PORT") {
    value.parse::<u16>().unwrap_or(DEFAULT_SERVER_PORT)
  } else {
    DEFAULT_SERVER_PORT
  };
  format!("{proto}://{server_addr}:{server_port}", proto = "http")
}

pub fn guess_mime_type_from_path(path: PathBuf) -> String {
  match path.extension().and_then(|ext| ext.to_str()) {
    Some("html") => "text/html",
    Some("css") => "text/css",
    Some("js") => "application/javascript",
    Some("png") => "image/png",
    Some("jpg") | Some("jpeg") => "image/jpeg",
    Some("gif") => "image/gif",
    _ => "application/octet-stream",
  }
  .to_string()
}
