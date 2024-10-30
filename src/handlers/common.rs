/// ### Handler for API "/"
#[utoipa::path(get, path = "/")]
pub async fn home() -> &'static str {
  tracing::debug!("GET :: /");
  "Let's quick chat with AnonymousChatBox"
}
