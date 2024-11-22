use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct FileResponse {
  pub name: String,
  pub file_path: String,
}
