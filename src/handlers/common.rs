/// ### Handler for API "/"
#[utoipa::path(get, path = "/")]
pub async fn home() -> &'static str {
  tracing::debug!("GET :: /");
  "Let's quick chat with AnonymousChatBox"
}

pub async fn fallback() -> &'static str {
  "The requested URL was not found on the server."
}
