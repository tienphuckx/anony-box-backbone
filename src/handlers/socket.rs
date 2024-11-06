use axum::{
  extract::{
    ws::{close_code::NORMAL, CloseFrame, Message, WebSocket},
    ConnectInfo, WebSocketUpgrade,
  },
  response::IntoResponse,
};
use axum_extra::{headers::UserAgent, TypedHeader};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{net::SocketAddr, ops::ControlFlow};
use tokio::time::Duration;

pub async fn ws_handler(
  ws: WebSocketUpgrade,
  user_agent: Option<TypedHeader<UserAgent>>,
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
  let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    user_agent.to_string()
  } else {
    "unknown".into()
  };
  tracing::info!("user agent: {user_agent} at {addr} connected");
  ws.on_upgrade(move |socket| handle_socket(socket, addr))
}
pub async fn handle_socket(mut socket: WebSocket, addr: SocketAddr) {
  if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
    tracing::debug!("Pinged {}....", addr);
  } else {
    tracing::error!("Could not send ping {}", addr);
  }

  let (mut sender, mut receiver) = socket.split();

  let mut send_task = tokio::spawn(async move {
    let n_msg = 20;
    for i in 0..n_msg {
      if sender
        .send(Message::Text(format!(
          "This is message from server : {}",
          i
        )))
        .await
        .is_err()
      {
        return i;
      }
      tokio::time::sleep(Duration::from_millis(300)).await;
    }
    tracing::info!("Send close message to {}", addr);
    if let Err(err) = sender
      .send(Message::Close(Some(CloseFrame {
        code: NORMAL,
        reason: "Good bye".into(),
      })))
      .await
    {
      tracing::error!(
        "Could not send close message to {} cause {}",
        addr,
        err.to_string()
      );
    }
    n_msg
  });

  let mut receive_task = tokio::spawn(async move {
    let mut cnt = 0;
    while let Some(Ok(msg)) = receiver.next().await {
      cnt += 1;
      if process_message(msg, addr).is_break() {
        break;
      }
    }
    cnt
  });

  tokio::select! {
    rv_a = (&mut send_task) =>{
      match rv_a{
        Ok(a) =>{
          tracing::debug!("{a} message sent to {addr}");

        }
        Err(a) =>{
          tracing::debug!("Error sending message {a:?}");
        }
      }
      receive_task.abort();
    },
    rv_b = (&mut receive_task) =>{
      match rv_b{
        Ok(b) => tracing::debug!("Received {b} message"),
        Err(b) => tracing::debug!("Error receiving message  {b:?}")
      }
      send_task.abort();
    }
  }
  tracing::info!("Websocket context {addr} destroyed");
}

fn process_message(msg: Message, addr: SocketAddr) -> ControlFlow<(), ()> {
  match msg {
    Message::Ping(v) => {
      tracing::debug!(">> {addr} send ping message {v:?}")
    }
    Message::Pong(v) => {
      tracing::debug!(">> {addr} send pong message {v:?}")
    }
    Message::Text(data) => {
      tracing::debug!(">> {addr} send text message {data:?}")
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
