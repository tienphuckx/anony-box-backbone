use std::{env, sync::Arc};
mod database;
mod errors;
mod extractors;
mod handlers;
mod payloads;
mod router;
mod services;
mod utils;

use diesel::{
  r2d2::{self, ConnectionManager, Pool},
  PgConnection,
};

use dotenvy::dotenv;
use tokio::{net::TcpListener, signal};
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

  let app = router::init_router().with_state(app_state);

  let listener = TcpListener::bind((server_address.as_str(), server_port))
    .await
    .expect("Cannot listen on address");
  tracing::info!("Server is listening on {}:{}", server_address, server_port);
  // println!("Server is listening on port {}", server_port);
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
  tracing::info!("Server is shutdown");
}

async fn shutdown_signal() {
  let ctrl_c = async {
    signal::ctrl_c()
      .await
      .expect("failed to install Ctrl+C handler");
  };

  #[cfg(unix)]
  let terminate = async {
    signal::unix::signal(signal::unix::SignalKind::terminate())
      .expect("failed to install signal handler")
      .recv()
      .await;
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
      _ = ctrl_c => {},
      _ = terminate => {}
  }
}
