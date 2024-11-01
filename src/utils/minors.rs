use axum_extra::extract::CookieJar;

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
