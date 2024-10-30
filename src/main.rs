use std::{env, sync::Arc};
mod database;
mod errors;
mod handlers;
mod msg_handlers;
mod payloads;
mod services;
mod user_handlers;
mod utils;

use axum::{
  routing::{get, post},
  Router,
};
use diesel::{
  r2d2::{self, ConnectionManager, Pool},
  PgConnection,
};

use payloads::{
  common::CommonResponse,
  groups::{GroupInfo, GroupListResponse, NewGroupForm},
  user::{NewUserRequest, UserResponse},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::msg_handlers::{get_latest_messages, get_latest_messages_by_code};
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use utils::constants::*;

fn config_logging() {
  let directives = format!("{level}", level = LevelFilter::DEBUG);
  let filter = EnvFilter::new(directives);
  let registry = tracing_subscriber::registry().with(filter);
  registry.with(tracing_subscriber::fmt::layer()).init();
}

pub struct AppState {
  pub db_pool: Pool<ConnectionManager<PgConnection>>,
}

pub fn init_router() -> Router<Arc<AppState>> {
  Router::new()
    .route("/", get(handlers::home))
    .route("/add-user-group", post(handlers::create_user_and_group)) // this api add new a user and new gr
    .route("/add-user", post(handlers::add_user)) //first: create a new user
    .route("/create-group", post(handlers::create_group_with_user)) // second: create a new group by user id
    .route("/send-msg", post(msg_handlers::send_msg))
    .route("/get-latest-messages", post(get_latest_messages))
    .route(
      "/get-latest-messages/:group_code",
      get(get_latest_messages_by_code),
    )
    .route("/add-user-doc", post(user_handlers::add_user_docs))
}

// Define Utoipa OpenAPI structure
#[derive(OpenApi)]
#[openapi(
  paths(
    handlers::create_user_and_group,
    user_handlers::add_user_docs,
    handlers::get_user_groups,
  ),
  components(schemas(NewGroupForm, NewUserRequest, UserResponse, CommonResponse<UserResponse>, GroupListResponse, GroupInfo))
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
  config_logging();
  dotenv().ok();
  let database_url = env::var("DATABASE_URL").expect("Database URL must be set");
  let server_address = env::var("SERVER_ADDRESS").unwrap_or(DEFAULT_SERVER_ADDRESS.to_string());
  let server_port = if let Ok(value) = env::var("SERVER_PORT") {
    value.parse::<u16>().expect("Server port must be a number")
  } else {
    8091
  };

  let pool_size = if let Ok(value) = env::var("MAXIMUM_POOL_SIZE") {
    value.parse::<u32>().expect("Pool size must be a number")
  } else {
    DEFAULT_POOL_SIZE
  };

  let manager = ConnectionManager::<PgConnection>::new(database_url);
  let db_pool = r2d2::Pool::builder()
    .max_size(pool_size)
    .build(manager)
    .expect("Failed to create connection pool");

  let app_state = Arc::new(AppState { db_pool });

  let app = init_router()
    .with_state(app_state)
    .merge(SwaggerUi::new("/swagger-ui").url("/api/docs/open-api.json", ApiDoc::openapi()));

  let listener = TcpListener::bind((server_address.as_str(), server_port))
    .await
    .expect("Cannot listen on address");
  tracing::info!("Server is listening on {}:{}", server_address, server_port);
  // println!("Server is listening on port {}", server_port);
  axum::serve(listener, app).await.unwrap();
}
