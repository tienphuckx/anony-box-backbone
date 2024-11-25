use serde::Serialize;
use utoipa::ToSchema;
#[derive(Serialize, Debug, ToSchema)]
pub enum ContentType {
  Text,
  Image,
  Audio,
  Video,
  Compression,
  Unknown,
}

impl From<&str> for ContentType {
  fn from(value: &str) -> Self {
    // Match on broad categories first, then handle specifics
    if value.starts_with("text") || value == "application/json" {
      Self::Text
    } else if value.starts_with("audio/") {
      Self::Audio
    } else if value.starts_with("video/") {
      Self::Video
    } else if value.starts_with("image/") {
      Self::Image
    } else if value.starts_with("application/") {
      match value {
        "application/zip" | "application/x-7z-compressed" | "application/vnd.rar" => {
          Self::Compression
        }
        _ => Self::Unknown,
      }
    } else {
      Self::Unknown
    }
  }
}

#[derive(Serialize, ToSchema)]
pub struct FileResponse {
  pub name: String,
  pub file_path: String,
  pub content_type: ContentType,
}
