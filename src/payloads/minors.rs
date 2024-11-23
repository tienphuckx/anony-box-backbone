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
    match value {
      "text/html" | "text/plain" | "text/csv" | "application/json" | "text/javascript" => {
        Self::Text
      }
      "audio/mpeg" | "audio/wav" => Self::Audio,
      "video/mp4" | "video/mpeg" | "video/webm" => Self::Video,
      "image/png" | "image/jpeg" | "image/webp" => Self::Image,
      "application/zip" | "application/x-7z-compressed" | "application/vnd.rar" => {
        Self::Compression
      }
      _ => Self::Unknown,
    }
  }
}

#[derive(Serialize, ToSchema)]
pub struct FileResponse {
  pub name: String,
  pub file_path: String,
  pub content_type: ContentType,
}
