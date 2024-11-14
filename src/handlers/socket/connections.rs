use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;
use tokio::sync::broadcast::Sender;

use crate::{payloads::socket::message::SMessageType, services, PoolPGConnectionType};

pub type ClientSessionsType = Lazy<Mutex<HashMap<i32, Sender<SMessageType>>>>;

pub static CLIENT_SESSIONS: ClientSessionsType =
  Lazy::new(|| Mutex::new(HashMap::<i32, Sender<SMessageType>>::new()));

pub fn send_message_event_to_group(
  conn: &mut PoolPGConnectionType,
  new_message: SMessageType,
  group_id: i32,
) -> Result<usize, ()> {
  let user_ids = services::user::get_user_ids_from_group(conn, group_id);
  if user_ids.is_err() {
    return Err(());
  }
  let user_ids = user_ids.unwrap();
  if user_ids.is_empty() {
    return Ok(0);
  }

  let mut count = 0;
  if let Some(active_connections) = get_connected_connections(user_ids) {
    for active_connection in active_connections {
      if active_connection.send(new_message.clone()).is_ok() {
        count += 1;
      }
    }
  }
  Ok(count)
}

fn get_connected_connections(user_ids: Vec<i32>) -> Option<Vec<Sender<SMessageType>>> {
  // let mut result = Vec::new();
  if let Ok(client_sessions) = CLIENT_SESSIONS.lock() {
    let result = client_sessions
      .iter()
      .filter(|session| user_ids.contains(&session.0))
      .map(|session| session.1.clone())
      .collect::<Vec<Sender<SMessageType>>>();
    return Some(result);
  }
  None
}
