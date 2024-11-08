use crate::{
  errors::ApiError,
  extractors::UserToken,
  payloads::socket::message::{SMessageContent, SMessageType},
  services::{group::check_user_join_group, message::create_new_message, user::get_user_by_code},
  AppState,
};
use axum::{
  extract::{
    ws::{Message, WebSocket},
    ConnectInfo, Path, State, WebSocketUpgrade,
  },
  response::IntoResponse,
};
use axum_extra::{headers::UserAgent, TypedHeader};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{net::SocketAddr, ops::ControlFlow, sync::Arc};
use tokio::sync::broadcast::{self, Sender};

#[derive(Clone, Copy)]
pub struct GroupSession {
  pub user_id: i32,
  pub group_id: i32,
}

pub async fn ws_group_handler(
  ws: WebSocketUpgrade,
  State(state): State<Arc<AppState>>,
  Path(group_id): Path<i32>,
  UserToken(token): UserToken,
  user_agent: Option<TypedHeader<UserAgent>>,
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, ApiError> {
  // For debugging
  let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    user_agent.to_string()
  } else {
    "unknown".into()
  };
  if token.is_none() {
    return Err(ApiError::Forbidden);
  }
  tracing::debug!("User agent: {user_agent} at {addr} connected");
  let conn = &mut state.db_pool.get().unwrap();

  // Validate user authentication and authorization
  let user = get_user_by_code(conn, &token.unwrap())
    .map_err(|_| ApiError::new_database_query_err("Failed to get user from user code"))?
    .ok_or(ApiError::NotFound("User".into()))?;

  if !check_user_join_group(conn, user.id, group_id).map_err(ApiError::DatabaseError)? {
    return Err(ApiError::Unauthorized);
  }
  let group_session = GroupSession {
    user_id: user.id,
    group_id,
  };

  Ok(ws.on_upgrade(move |socket| handle_socket(socket, addr, state, group_session)))
}
pub async fn handle_socket(
  socket: WebSocket,
  addr: SocketAddr,
  app_state: Arc<AppState>,
  group_session: GroupSession,
) {
  let GroupSession { group_id, .. } = group_session;
  let (mut socket_sender, mut socket_receiver) = socket.split();

  let mut shared_group_tx = {
    let mut group_txs = app_state.group_txs.lock().await;
    match group_txs.get(&group_id) {
      Some(txs) => txs.clone(),
      None => {
        let (tx, _rx) = broadcast::channel(1000);
        group_txs.insert(group_id, tx.clone());
        tx
      }
    }
  };
  let mut shared_group_rx = shared_group_tx.subscribe();

  // Propagate message events to all subscribe clients
  let mut propagate_task = tokio::spawn(async move {
    while let Ok(msg) = shared_group_rx.recv().await {
      tracing::debug!("Propagate message from group {group_id} to client");
      if let Err(err) = socket_sender
        .send(Message::Text(serde_json::to_string(&msg).unwrap()))
        .await
      {
        tracing::info!("Stop handling propagate message to client {addr}");
        tracing::error!(
          "Failed to send message to client {}, cause: {}",
          addr,
          err.to_string()
        );
        break;
      }
    }
  });
  // Received message from client and process message
  let mut receive_task = tokio::spawn(async move {
    while let Some(Ok(msg)) = socket_receiver.next().await {
      if process_message(
        msg,
        addr,
        app_state.clone(),
        group_session,
        &mut shared_group_tx,
      )
      .is_break()
      {
        tracing::info!("Stop handling message from {addr}");
        break;
      }
    }
  });
  // Abort the other task, if any one of the task exists
  tokio::select! {
    _p_t = (&mut propagate_task) =>{
      receive_task.abort();
    },
    _r_t = (&mut receive_task) =>{
      propagate_task.abort();
    }
  }
}

fn process_message(
  msg: Message,
  addr: SocketAddr,
  state: Arc<AppState>,
  group_session: GroupSession,
  shared_group_sender: &mut Sender<SMessageType>,
) -> ControlFlow<(), ()> {
  let conn = &mut state.db_pool.get().unwrap();
  match msg {
    Message::Ping(v) => {
      tracing::debug!(">> {addr} send ping message {v:?}")
    }
    Message::Pong(v) => {
      tracing::debug!(">> {addr} send pong message {v:?}")
    }
    Message::Text(raw_str) => {
      let rs = serde_json::from_slice::<SMessageType>(raw_str.as_bytes());
      if let Err(err) = rs {
        tracing::debug!("Parse json error: {} ", err.to_string());
        tracing::debug!("Not support socket message type");
        return ControlFlow::Break(());
      }
      match rs.unwrap() {
        SMessageType::Send(s_new_message) => {
          tracing::debug!(">> SEND message: {s_new_message:?}");
          let insert_message =
            s_new_message.build_new_message(group_session.user_id, group_session.group_id);
          let insertion_rs = create_new_message(conn, insert_message);

          if insertion_rs.is_err() {
            return ControlFlow::Break(());
          }

          if shared_group_sender
            .send(SMessageType::Receive(SMessageContent::from(
              insertion_rs.unwrap(),
            )))
            .is_err()
          {
            tracing::error!("Cannot send RECEIVED message to client {addr}");
          }
        }
        SMessageType::Receive(_message_content) => {}
        SMessageType::Delete(_message_ids) => {}

        _ => {
          tracing::debug!("Cannot handle message ");
        }
      }
      tracing::debug!(">> {addr} send text message {raw_str:?}");
    }
    Message::Binary(data) => {
      tracing::debug!(">> {addr} send binary message {data:?}")
    }
    Message::Close(frame) => {
      if let Some(cf) = frame {
        println!(
          ">>> {} sent close with code {} and reason `{}`",
          addr, cf.code, cf.reason
        );
      } else {
        println!(">>> {addr} somehow sent close message without CloseFrame");
      }
      return ControlFlow::Break(());
    }
  }

  ControlFlow::Continue(())
}
