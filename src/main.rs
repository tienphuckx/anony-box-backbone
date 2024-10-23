use std::{env, sync::Arc};
mod database;
mod errors;
mod handlers;
mod payloads;
mod utils;
use axum::{
  routing::{get, post},
  Router,
};
use diesel::{
  r2d2::{self, ConnectionManager, Pool},
  PgConnection,
};
use dotenvy::dotenv;
use tokio::net::TcpListener;
use utils::constants::*;

pub struct AppState {
  pub db_pool: Pool<ConnectionManager<PgConnection>>,
}
pub fn init_router() -> Router<Arc<AppState>> {
  Router::new()
    .route("/home", get(handlers::home))
    .route("/new-group", post(handlers::create_group))
}

#[tokio::main]
async fn main() {
  dotenv().ok();
  let database_url = env::var("DATABASE_URL").expect("Database url must be set");
  let server_address = env::var("SERVER_ADDRESS").unwrap_or(DEFAULT_SERVER_ADDRESS.to_string());

  let server_port = if let Ok(value) = env::var("SERVER_PORT") {
    value.parse::<u16>().expect("Server port must be a number")
  } else {
    DEFAULT_SERVER_PORT
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
  let app = init_router().with_state(app_state);

  let listener = TcpListener::bind((server_address, server_port))
    .await
    .expect("Cannot listen on address");
  println!("Server is listening on port {}", server_port);
  axum::serve(listener, app).await.unwrap();
}
