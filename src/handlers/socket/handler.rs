use crate::{
  database::models::MessageStatus,
  errors::ApiError,
  handlers::socket::{
    connections::{self, send_message_event_to_group, CLIENT_SESSIONS},
    structs::ClientSession,
  },
  payloads::{
    messages::AttachmentPayload,
    socket::{
      common::ResultMessage,
      message::{
        AuthenticationStatusCode, MessagesData, SMessageContent, SMessageEdit, SMessageType,
      },
    },
  },
  services::{
    self, group::check_user_join_group, message::create_new_message, user::get_user_by_code,
  },
  AppState, PoolPGConnectionType,
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

  let authenticated_rs = authenticate(first_message, app_state.clone(), &mut current_sender, addr);

  if authenticated_rs.is_err() {
    tracing::info!("Client {addr} authenticated failed");
    return;
  }
  let mut client_session = authenticated_rs.unwrap();
  CLIENT_SESSIONS
    .lock()
    .unwrap()
    .insert(client_session.user_id, shared_tx.clone());

  // Received message from client and process message
  let mut receiving_task = tokio::spawn(async move {
    while let Some(Ok(msg)) = socket_receiver.next().await {
      if process_message(
        msg,
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
  addr: SocketAddr,
) -> Result<ClientSession, ()> {
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
          return Ok(ClientSession {
            user_id: user.id,
            username: user.username,
            addr,
          });
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
  app_state: Arc<AppState>,
  client_session: &mut ClientSession,
  current_sender: &mut Sender<SMessageType>,
) -> ControlFlow<(), ()> {
  let conn = &mut app_state.db_pool.get().unwrap();
  tracing::debug!(">> Client {} SEND message", client_session.addr);
  match msg {
    Message::Ping(v) => {
      tracing::debug!(">> {} send ping message {v:?}", client_session.addr)
    }
    Message::Pong(v) => {
      tracing::debug!(">> {} send pong message {v:?}", client_session.addr)
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
          if let Some(value) =
            process_send_message(conn, client_session, s_new_message, current_sender)
          {
            return value;
          }
        }
        SMessageType::DeleteMessage(delete_message_data) => {
          process_delete_message(conn, client_session, current_sender, delete_message_data);
        }
        SMessageType::EditMessage(edit_message) => {
          process_update_message(conn, current_sender, edit_message);
        }
        SMessageType::SeenMessages(messages_request) => {
          process_seen_messages(conn, client_session, current_sender, messages_request);
        }
        _ => {
          tracing::debug!("Cannot handle message type");
        }
      }
      tracing::debug!(">> {} send text message {:?}", client_session.addr, raw_str);
    }
    Message::Binary(data) => {
      tracing::debug!(">> {} send binary message {:?}", client_session.addr, data)
    }
    Message::Close(frame) => {
      if let Some(cf) = frame {
        tracing::debug!(
          ">>> {} sent close with code {} and reason `{}`",
          client_session.addr,
          cf.code,
          cf.reason
        );
      } else {
        tracing::debug!(
          ">>> {} somehow sent close message without CloseFrame",
          client_session.addr
        );
      }
      return ControlFlow::Break(());
    }
  }
  ControlFlow::Continue(())
}

fn process_update_message(
  conn: &mut PoolPGConnectionType,
  current_sender: &mut Sender<SMessageType>,
  edit_message: SMessageEdit,
) {
  let SMessageEdit {
    message_id,
    group_id,
    ..
  } = edit_message.clone();
  let message_rs = services::message::update_message(conn, message_id, edit_message.into());
  if let Err(ref err) = message_rs {
    let _ = current_sender.send(SMessageType::EditMessageResponse(ResultMessage::new(
      1,
      &format!("Failed to update message, {}", err.to_string()),
    )));
  } else {
    let _ = send_message_event_to_group(
      conn,
      SMessageType::EditMessageData(SMessageContent::from(message_rs.unwrap())),
      group_id,
    );
  }
}

fn process_delete_message(
  conn: &mut PoolPGConnectionType,
  client_session: &mut ClientSession,
  current_sender: &mut Sender<SMessageType>,
  MessagesData {
    group_id,
    message_ids,
  }: MessagesData,
) {
  tracing::debug!(">> Client {} DELETE message", client_session.addr);
  let invalid_message_ids =
    services::message::check_owner_of_messages(conn, client_session.user_id, &message_ids);
  if let Err(ref err) = invalid_message_ids {
    tracing::error!("Error when check owner of messages: {}", err.to_string());

    let _ = current_sender.send(SMessageType::DeleteMessageResponse(ResultMessage::new(
      1,
      "There is an error, please try later",
    )));
  }
  let invalid_message_ids = invalid_message_ids.unwrap();
  if !invalid_message_ids.is_empty() {
    let _ = current_sender.send(SMessageType::DeleteMessageResponse(ResultMessage::new(
      2,
      format!(
        "Invalid message ids, maybe user are not owner of messages: {:?}",
        invalid_message_ids
      )
      .as_str(),
    )));
  } else {
    if let Ok(true) = services::message::delete_messages(conn, &message_ids) {
      let _ = send_message_event_to_group(
        conn,
        SMessageType::DeleteMessageEvent(MessagesData {
          group_id,
          message_ids,
        }),
        group_id,
      );
    } else {
      let _ = current_sender.send(SMessageType::DeleteMessageResponse(ResultMessage::new(
        2,
        "Failed to delete message, maybe one of messages ids is not found",
      )));
    }
  }
}

fn process_send_message(
  conn: &mut PoolPGConnectionType,
  client_session: &mut ClientSession,
  s_new_message: crate::payloads::socket::message::SNewMessage,
  current_sender: &mut Sender<SMessageType>,
) -> Option<ControlFlow<()>> {
  tracing::debug!(
    ">> Client {} SEND message: {:?}",
    client_session.addr,
    s_new_message
  );
  if let Ok(rs) = check_user_join_group(conn, client_session.user_id, s_new_message.group_id) {
    if rs {
      let insert_message = s_new_message.build_new_message(client_session.user_id);
      let insertion_rs = create_new_message(conn, insert_message);

      if insertion_rs.is_err() {
        return Some(ControlFlow::Break(()));
      }
      let inserted_message = insertion_rs.unwrap();
      let mut inserted_attachment_payloads = None;
      if let Some(attachments) = s_new_message.attachments {
        let new_attachments = attachments
          .iter()
          .map(|e| AttachmentPayload::into_new(e, inserted_message.id))
          .collect();

        match services::attachment::create_attachments(conn, new_attachments) {
          Ok(inserted_attachments) => {
            inserted_attachment_payloads = Some(
              inserted_attachments
                .iter()
                .map(|e| AttachmentPayload::from(e.clone()))
                .collect::<Vec<AttachmentPayload>>(),
            );
          }
          Err(err) => {
            tracing::error!(
              "Failed to create new attachments of message id {}: {} ",
              inserted_message.id,
              err.to_string()
            )
          }
        }
      }
      let mut message_content = SMessageContent::from(inserted_message);
      message_content.attachments = inserted_attachment_payloads;
      message_content.username = Some(client_session.username.clone());
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
      tracing::debug!(
        "Client {} did  not joined group {}",
        client_session.addr,
        s_new_message.group_id
      );
      if current_sender
        .send(SMessageType::AuthenticateResponse(
          AuthenticationStatusCode::NoPermission.into(),
        ))
        .is_err()
      {
        tracing::error!(
          "Failed to send AuthenticateResponse to client {}",
          client_session.addr
        );
      }
    }
  } else {
    tracing::debug!(
      "Client {} does not have permission to access group {}",
      client_session.addr,
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
  None
}

fn process_seen_messages(
  conn: &mut PoolPGConnectionType,
  client_session: &mut ClientSession,
  current_sender: &mut Sender<SMessageType>,
  MessagesData {
    group_id,
    message_ids,
  }: MessagesData,
) {
  // check current user joined the group
  if let Ok(joined) = check_user_join_group(conn, client_session.user_id, group_id) {
    if !joined {
      let _ = current_sender.send(SMessageType::SeenMessagesResponse(ResultMessage::new(
        1,
        "User hasn't joined the group",
      )));
      return;
    }
  } else {
    let _ = current_sender.send(SMessageType::SeenMessagesResponse(ResultMessage::new(
      2,
      "Failed to check user joined group, try again later",
    )));
    return;
  }
  // check all messages in groups

  let messages_rs = services::message::get_messages_from_ids(conn, &message_ids);

  if let Err(_err) = messages_rs {
    let _ = current_sender.send(SMessageType::SeenMessagesResponse(ResultMessage::new(
      3,
      "Failed to get message from ids, try again later",
    )));
    return;
  }

  let messages = messages_rs.unwrap();

  if messages.iter().any(|message| message.group_id != group_id) {
    let _ = current_sender.send(SMessageType::SeenMessagesResponse(ResultMessage::new(
      4,
      &format!("One of messages is not belong to group {}", group_id),
    )));
    return;
  }

  // process seen messages
  if let Err(_) = services::message::change_messages_status(conn, &message_ids, MessageStatus::Seen)
  {
    let _ = current_sender.send(SMessageType::SeenMessagesResponse(ResultMessage::new(
      5,
      "Failed to change messages status, try again later",
    )));
    return;
  }

  let _ = send_message_event_to_group(
    conn,
    SMessageType::SeenMessagesEvent(MessagesData {
      group_id,
      message_ids,
    }),
    group_id,
  );
  // propagate seen message to active client connections
}
