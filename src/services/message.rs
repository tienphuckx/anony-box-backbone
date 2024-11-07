use diesel::{RunQueryDsl, SelectableHelper};

use crate::{
  database::models::{self, Message, NewMessage},
  errors::DBError,
  PoolPGConnectionType,
};

pub fn create_new_message(
  conn: &mut PoolPGConnectionType,
  new_message: NewMessage,
) -> Result<Message, DBError> {
  use crate::database::schema::messages::dsl::*;
  let message = diesel::insert_into(messages)
    .values(new_message)
    .returning(models::Message::as_returning())
    .get_result::<models::Message>(conn)
    .map_err(|err| {
      tracing::error!("Failed to insert new message: {}", err.to_string());
      return DBError::QueryError("Failed to insert new message".into());
    })?;
  Ok(message)
}
