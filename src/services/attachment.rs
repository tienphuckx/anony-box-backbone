use diesel::{RunQueryDsl, SelectableHelper};

use crate::{
  database::models::{self, Attachment, NewAttachment},
  errors::DBError,
  PoolPGConnectionType,
};
#[allow(dead_code)]
pub fn create_attachment(
  conn: &mut PoolPGConnectionType,
  new_attachment: NewAttachment,
) -> Result<Attachment, DBError> {
  use crate::database::schema::attachments::dsl::*;
  let attachment = diesel::insert_into(attachments)
    .values(new_attachment)
    .returning(models::Attachment::as_returning())
    .get_result::<models::Attachment>(conn)
    .map_err(|err| {
      tracing::error!("Failed to insert new attachment: {}", err.to_string());
      return DBError::QueryError("Failed to insert new attachment".into());
    })?;
  Ok(attachment)
}

pub fn create_attachments(
  conn: &mut PoolPGConnectionType,
  new_attachments: Vec<NewAttachment>,
) -> Result<Vec<Attachment>, DBError> {
  use crate::database::schema::attachments::dsl::*;
  let attachment = diesel::insert_into(attachments)
    .values(new_attachments)
    .returning(models::Attachment::as_returning())
    .get_results::<models::Attachment>(conn)
    .map_err(|err| {
      tracing::error!("Failed to insert new attachment: {}", err.to_string());
      return DBError::QueryError("Failed to insert new attachment".into());
    })?;
  Ok(attachment)
}
