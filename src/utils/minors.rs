use axum_extra::extract::CookieJar;

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
pub mod custom_serde {
  use std::str::FromStr;

  use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize_naive_datetime<S>(
    datetime: &NaiveDateTime,
    serializer: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&s)
  }

  pub fn serialize_with_date_time_utc<S>(
    datetime: &DateTime<Utc>,
    serializer: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&datetime.to_rfc3339())
  }

  pub fn deserialize_with_date_time_utc<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
  where
    D: Deserializer<'de>,
  {
    let raw_str = String::deserialize(deserializer)?;
    let result = DateTime::from_str(&raw_str);
    if let Ok(date_time) = result {
      Ok(date_time)
    } else {
      Err(serde::de::Error::custom("Not a valid Utc Datetime format"))
    }
  }

  pub fn deserialize_with_naive_date_option<'de, D>(
    deserializer: D,
  ) -> Result<Option<NaiveDate>, D::Error>
  where
    D: Deserializer<'de>,
  {
    // Attempt to deserialize an optional string
    let opt: Option<&str> = Option::deserialize(deserializer)?;
    tracing::debug!("deserialize option date: {:?}", opt);

    // If there is Some value, try to parse it as a NaiveDate, else return None
    match opt {
      Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(Some)
        .map_err(serde::de::Error::custom),
      None => Ok(None),
    }
  }
}
