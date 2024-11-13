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
  let active_connections = get_connected_connections(user_ids);
  let mut count = 0;
  for active_connection in active_connections {
    if active_connection.send(new_message.clone()).is_ok() {
      count += 1;
    }
  }
  Ok(count)
}

fn get_connected_connections(user_ids: Vec<i32>) -> Vec<Sender<SMessageType>> {
  let mut result = Vec::new();
  if let Ok(client_sessions) = CLIENT_SESSIONS.lock() {
    for user_id in user_ids {
      if client_sessions.contains_key(&user_id) {
        result.push(client_sessions.get(&user_id).unwrap().clone());
      }
    }
  }
  result
}
