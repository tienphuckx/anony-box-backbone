use crate::{
  errors::ApiError,
  handlers::socket::{
    connections::{self, CLIENT_SESSIONS},
    structs::ClientSession,
  },
  payloads::socket::message::{AuthenticationStatusCode, SMessageContent, SMessageType},
  services::{group::check_user_join_group, message::create_new_message, user::get_user_by_code},
  AppState,
};
use axum::{
  extract::{
    ws::{Message, WebSocket},
    ConnectInfo, State, WebSocketUpgrade,
  },
  response::IntoResponse,
};
use axum_extra::{headers::UserAgent, TypedHeader};
use futures::{sink::SinkExt, stream::StreamExt};

use std::{net::SocketAddr, ops::ControlFlow, sync::Arc, time::Duration};
use tokio::{
  sync::broadcast::{self, Sender},
  time::timeout,
};

pub async fn ws_handler(
  ws: WebSocketUpgrade,
  State(state): State<Arc<AppState>>,
  user_agent: Option<TypedHeader<UserAgent>>,
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, ApiError> {
  // Logging connection's user agent
  let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    user_agent.to_string()
  } else {
    "unknown".into()
  };
  tracing::debug!("User agent: {user_agent} at {addr} connected");
  Ok(ws.on_upgrade(move |socket| handle_socket(socket, addr, state)))
}
pub async fn handle_socket(socket: WebSocket, addr: SocketAddr, app_state: Arc<AppState>) {
  let (mut socket_sender, mut socket_receiver) = socket.split();
  // Shared channel for receiving data from other channel then sending to current connection
  let (shared_tx, mut shared_rx) = broadcast::channel::<SMessageType>(1003);

  // Receive all data from shared channel then sending to current connection
  let mut sending_task = tokio::spawn(async move {
    while let Ok(msg) = shared_rx.recv().await {
      // tracing::debug!("Propagate message from group {group_id} to client");
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
  // Sender and Receiver serve for current connection
  let (mut current_sender, mut current_receiver) = broadcast::channel::<SMessageType>(3);
  let share_tx_clone = shared_tx.clone();
  // Current channel receiver receives data then propagate to shared channel
  tokio::spawn(async move {
    while let Ok(msg) = current_receiver.recv().await {
      let _ = share_tx_clone.send(msg);
    }
  });

  // Handle first authentication message
  let timeout_rs = timeout(Duration::from_secs(10), socket_receiver.next()).await;
  if let Err(_err) = &timeout_rs {
    tracing::info!("Client authenticate is timeout");
    if current_sender
      .send(SMessageType::AuthenticateResponse(
        AuthenticationStatusCode::Timeout.into(),
      ))
      .is_err()
    {
      tracing::error!("Failed to send Timeout message to client");
    }
  }
  let first_message_op = timeout_rs.unwrap();
  if first_message_op.is_none() {
    tracing::info!("Stream has been closed, so cannot read");
    return;
  }
  let first_message_rs = first_message_op.unwrap();
  if first_message_rs.is_err() {
    tracing::info!("Failed to received first authenticate message");
    return;
  }
  let first_message = first_message_rs.unwrap();

  let authenticated_rs = authenticate(first_message, app_state.clone(), &mut current_sender, &addr);

  if authenticated_rs.is_err() {
    tracing::info!("Client {addr} authenticated failed");
    return;
  }

  let mut client_session = ClientSession {
    user_id: authenticated_rs.unwrap(),
  };

  CLIENT_SESSIONS
    .lock()
    .unwrap()
    .insert(client_session.user_id, shared_tx.clone());

  // Received message from client and process message
  let mut receiving_task = tokio::spawn(async move {
    while let Some(Ok(msg)) = socket_receiver.next().await {
      if process_message(
        msg,
        addr,
        app_state.clone(),
        &mut client_session,
        &mut current_sender,
      )
      .await
      .is_break()
      {
        tracing::info!("Stop handling message from {addr}");
        break;
      }
    }
  });
  // Abort the other task, if any one of the task exists
  tokio::select! {
    _p_t = (&mut sending_task) =>{
      receiving_task.abort();
    },
    _r_t = (&mut receiving_task) =>{
      sending_task.abort();
    }
  }
}

/// Authenticate first message
///
/// If authenticating successfully the user_id result will be returned, unless return error
fn authenticate(
  msg: Message,
  state: Arc<AppState>,
  current_sender: &mut Sender<SMessageType>,
  addr: &SocketAddr,
) -> Result<i32, ()> {
  match msg {
    Message::Text(raw_str) => {
      let conn = &mut state.db_pool.get().unwrap();
      let rs = serde_json::from_slice::<SMessageType>(raw_str.as_bytes());
      if let Err(err) = rs {
        tracing::debug!("Not support socket message type: {}", err.to_string());
        if current_sender
          .send(SMessageType::UnSupportMessage(
            "Unsupported Message Format".into(),
          ))
          .is_err()
        {
          tracing::error!("Failed to send unsupported message type message");
        }
        return Err(());
      }
      match rs.unwrap() {
        SMessageType::Authenticate(user_code) => {
          // Validate user authentication and authorization
          let user_rs = get_user_by_code(conn, &user_code);
          if let Err(_err) = user_rs {
            if current_sender
              .send(SMessageType::AuthenticateResponse(
                AuthenticationStatusCode::Other.into(),
              ))
              .is_err()
            {
              tracing::error!("Failed to send authenticate result message");
            };
            return Err(());
          }
          let user_op = user_rs.unwrap();
          if let None = user_op {
            if current_sender
              .send(SMessageType::AuthenticateResponse(
                AuthenticationStatusCode::ExpireOrNotFound.into(),
              ))
              .is_err()
            {
              tracing::error!("Failed to send authenticate result message");
            }
            return Err(());
          }
          let user = user_op.unwrap();

          if current_sender
            .send(SMessageType::AuthenticateResponse(
              AuthenticationStatusCode::Success.into(),
            ))
            .is_err()
          {
            tracing::error!("Failed to send authenticate successfully message");
          };
          tracing::debug!("Client {addr} authenticated successfully");
          return Ok(user.id);
        }

        _ => {
          tracing::debug!("Cannot handle message ");
        }
      }
      tracing::debug!(">> {addr} send text message {raw_str:?}");
    }
    _ => {
      tracing::debug!("Only supports authenticated text message type");
      let _ = current_sender.send(SMessageType::AuthenticateResponse(
        AuthenticationStatusCode::UnsupportedMessageType.into(),
      ));
    }
  }
  Err(())
}

async fn process_message(
  msg: Message,
  addr: SocketAddr,
  app_state: Arc<AppState>,
  client_session: &mut ClientSession,
  current_sender: &mut Sender<SMessageType>,
) -> ControlFlow<(), ()> {
  let conn = &mut app_state.db_pool.get().unwrap();

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
        tracing::debug!("Not support socket message type: {}", err.to_string());
        if current_sender
          .send(SMessageType::UnSupportMessage(
            "Unsupported Message Format".into(),
          ))
          .is_err()
        {
          tracing::error!("Failed to send unsupported message type message");
        }
        return ControlFlow::Break(());
      }
      match rs.unwrap() {
        SMessageType::Send(s_new_message) => {
          tracing::debug!(">> Client {addr} SEND message: {s_new_message:?}");
          if let Ok(rs) =
            check_user_join_group(conn, client_session.user_id, s_new_message.group_id)
          {
            if rs {
              let insert_message = s_new_message.build_new_message(client_session.user_id);
              let insertion_rs = create_new_message(conn, insert_message);

              if insertion_rs.is_err() {
                return ControlFlow::Break(());
              }
              let message_content = SMessageContent::from(insertion_rs.unwrap());
              let send_rs = connections::send_message_event_to_group(
                conn,
                SMessageType::Receive(message_content),
                s_new_message.group_id,
              );
              if send_rs.is_err() {
                tracing::error!("Failed to send message event to group");
              } else {
                tracing::debug!("Send new message to {} clients", send_rs.unwrap());
              }
            } else {
            }
          } else {
            tracing::debug!(
              "Client {} does not have permission to access group {}",
              addr,
              s_new_message.group_id
            );
            if current_sender
              .send(SMessageType::AuthenticateResponse(
                AuthenticationStatusCode::NoPermission.into(),
              ))
              .is_err()
            {
              tracing::error!("Failed to send AuthenticateResponse to client");
            }
          }
        }
        SMessageType::Receive(_message_content) => {}
        SMessageType::Delete(_message_ids) => {}

        _ => {
          tracing::debug!("Cannot handle message type");
        }
      }
      tracing::debug!(">> {addr} send text message {raw_str:?}");
    }
    Message::Binary(data) => {
      tracing::debug!(">> {addr} send binary message {data:?}")
    }
    Message::Close(frame) => {
      if let Some(cf) = frame {
        tracing::debug!(
          ">>> {} sent close with code {} and reason `{}`",
          addr,
          cf.code,
          cf.reason
        );
      } else {
        tracing::debug!(">>> {addr} somehow sent close message without CloseFrame");
      }
      return ControlFlow::Break(());
    }
  }
  ControlFlow::Continue(())
}
