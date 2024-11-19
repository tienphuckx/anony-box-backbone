use std::net::SocketAddr;

#[derive(Clone)]
pub struct ClientSession {
  pub user_id: i32,
  pub username: String,
  pub addr: SocketAddr,
}
