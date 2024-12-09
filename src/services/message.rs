use chrono::{NaiveDateTime, NaiveTime, Utc};
use diesel::{
  pg::Pg, prelude::Queryable, BoolExpressionMethods, ExpressionMethods, JoinOnDsl,
  NullableExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
  TextExpressionMethods,
};
use uuid::Uuid;

use crate::{
  database::{
    models::{self, AttachmentTypeEnum, Message, MessageStatus, MessageTypeEnum, NewMessage},
    schema::{
      self, attachments,
      messages::{self},
      users,
    },
  },
  errors::DBError,
  payloads::{
    common::PageRequest,
    messages::{
      AttachmentPayload, MessageFilterParams, MessageSortParams, MessageWithUser, UpdateMessage,
    },
  },
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

#[derive(Queryable, Debug, Clone)]
pub struct MessageWithAttachmentRaw {
  pub message_uuid: Uuid,
  pub id: i32,
  pub content: Option<String>,
  pub message_type: MessageTypeEnum,
  pub status: MessageStatus,
  pub created_at: NaiveDateTime,
  pub updated_at: Option<NaiveDateTime>,
  pub user_id: i32,
  pub user_name: String,
  pub attachment_id: Option<i32>,
  pub url: Option<String>,
  pub attachment_type: Option<AttachmentTypeEnum>,
}

pub fn get_messages(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
  page: &PageRequest,
  message_filters: &MessageFilterParams,
  message_sorts: MessageSortParams,
) -> Result<Vec<MessageWithUser>, DBError> {
  let mut query = messages::table.into_boxed();

  query = query.filter(messages::group_id.eq(group_id));

  if let Some(content_type_val) = &message_filters.message_type {
    query = query.filter(messages::message_type.eq(content_type_val));
  }

  if let Some(ref content_val) = message_filters.content {
    query = query.filter(messages::content.like(format!("%{}%", content_val)));
  }
  if let Some(ref status_val) = message_filters.status {
    query = query.filter(messages::status.eq(status_val));
  }

  if let Some(from) = message_filters.from_date {
    let naive_datetime = NaiveDateTime::new(from, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    query = query.filter(messages::created_at.ge(naive_datetime));
  }
  if let Some(to) = message_filters.to_date {
    let naive_datetime = NaiveDateTime::new(to, NaiveTime::from_hms_opt(23, 59, 59).unwrap());
    query = query.filter(messages::created_at.le(naive_datetime));
  }

  let (offset, limit) = page.get_offset_and_limit();
  query = query.limit(limit as i64).offset(offset as i64);

  if let Some(created_at_sort) = message_sorts.created_at_sort {
    match created_at_sort {
      crate::payloads::common::OrderBy::ASC => query = query.order_by(messages::created_at.asc()),
      crate::payloads::common::OrderBy::DESC => query = query.order_by(messages::created_at.desc()),
    }
  }
  tracing::debug!("{}", diesel::debug_query::<Pg, _>(&query));

  let raw_results: Vec<MessageWithAttachmentRaw> = query
    .inner_join(users::table.on(users::id.eq(messages::user_id)))
    .left_join(
      schema::attachments::table.on(schema::messages::id.eq(schema::attachments::message_id)),
    )
    .select((
      messages::message_uuid,
      messages::id,
      messages::content.nullable(),
      messages::message_type,
      messages::status,
      messages::created_at,
      messages::updated_at,
      messages::user_id,
      users::username,
      attachments::id.nullable(),
      attachments::url.nullable(),
      attachments::attachment_type.nullable(),
    ))
    .load::<MessageWithAttachmentRaw>(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to load messages for group_id {}: {:?}",
        group_id,
        err
      );
      DBError::QueryError(format!("Error loading messages: {:?}", err))
    })?;

  let rs = map_raw_messages_to_payload(raw_results);
  Ok(rs)
}

fn map_raw_messages_to_payload(raw_results: Vec<MessageWithAttachmentRaw>) -> Vec<MessageWithUser> {
  let mut grouped_messages: std::collections::HashMap<i32, MessageWithUser> =
    std::collections::HashMap::new();

  for ref row in raw_results {
    let entry = grouped_messages.entry(row.id).or_insert_with(|| {
      let mut message = MessageWithUser::from(row.clone());
      message.attachments = Some(Vec::new());
      message
    });

    // If the row has an attachment, add it to the message's attachments
    if let Some(attachment_id) = row.attachment_id {
      entry.attachments.as_mut().unwrap().push(AttachmentPayload {
        id: attachment_id,
        url: row.url.clone().unwrap_or_default(),
        attachment_type: row.attachment_type.clone().unwrap_or_default(),
      });
    }
  }

  let rs: Vec<MessageWithUser> = grouped_messages
    .values()
    .map(|value| value.clone())
    .collect();
  rs
}

pub fn get_count_messages(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
  message_filters: MessageFilterParams,
) -> Result<i64, DBError> {
  let mut query = messages::table
    .inner_join(users::table.on(users::id.eq(messages::user_id)))
    .into_boxed();

  query = query.filter(messages::group_id.eq(group_id));
  // Filter by content type if provided
  if let Some(content_type_val) = &message_filters.message_type {
    query = query.filter(messages::message_type.eq(content_type_val));
  }

  // Filter by content if provided
  if let Some(ref content_val) = message_filters.content {
    query = query.filter(messages::content.like(format!("%{}%", content_val)));
  }

  // Filter by date range if provided
  if let Some(from) = message_filters.from_date {
    let naive_datetime = NaiveDateTime::new(from, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    query = query.filter(messages::created_at.ge(naive_datetime));
  }
  if let Some(to) = message_filters.to_date {
    let naive_datetime = NaiveDateTime::new(to, NaiveTime::from_hms_opt(23, 59, 59).unwrap());
    query = query.filter(messages::created_at.le(naive_datetime));
  }

  tracing::debug!("{}", diesel::debug_query::<Pg, _>(&query));

  let messages_count = query.count().get_result::<i64>(conn).map_err(|err| {
    tracing::error!(
      "Failed to get messages count for group_id {}: {:?}",
      group_id,
      err
    );
    DBError::QueryError(format!("Error get messages count: {:?}", err))
  })?;

  Ok(messages_count)
}

pub fn get_latest_messages_from_group(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
) -> Result<Vec<MessageWithUser>, DBError> {
  // Fetch messages (limit to latest messages if needed)
  let raw_results = messages::table
    .filter(messages::group_id.eq(group_id))
    .inner_join(users::table.on(users::id.eq(messages::user_id)))
    .left_join(
      schema::attachments::table.on(schema::messages::id.eq(schema::attachments::message_id)),
    )
    .order(messages::created_at.asc())
    .limit(10)
    .select((
      messages::message_uuid,
      messages::id,
      messages::content.nullable(),
      messages::message_type,
      messages::status,
      messages::created_at,
      messages::updated_at,
      messages::user_id,
      users::username,
      attachments::id.nullable(),
      attachments::url.nullable(),
      attachments::attachment_type.nullable(),
    ))
    .load::<MessageWithAttachmentRaw>(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to load messages for group_id {}: {:?}",
        group_id,
        err
      );
      DBError::QueryError(format!("Error loading messages: {:?}", err))
    })?;

  let rs = map_raw_messages_to_payload(raw_results);
  Ok(rs)
}

pub fn delete_message(conn: &mut PoolPGConnectionType, message_id: i32) -> Result<bool, DBError> {
  use crate::database::schema::messages;
  let affected_rows = diesel::delete(messages::table)
    .filter(messages::id.eq(message_id))
    .execute(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to get latest message {}: {}",
        message_id,
        err.to_string()
      );
      return DBError::QueryError("Failed to get latest message".into());
    })?;
  if affected_rows > 0 {
    Ok(true)
  } else {
    Ok(false)
  }
}

pub fn get_message(
  conn: &mut PoolPGConnectionType,
  message_id: i32,
) -> Result<Option<Message>, DBError> {
  use crate::database::schema::messages;
  Ok(
    messages::table
      .find(message_id)
      .select(Message::as_select())
      .get_result::<Message>(conn)
      .optional()
      .map_err(|err| {
        tracing::error!("Failed to get message {}: {}", message_id, err.to_string());
        return DBError::QueryError("Failed to get message".into());
      })?,
  )
}

pub fn get_messages_from_ids(
  conn: &mut PoolPGConnectionType,
  message_ids: &Vec<i32>,
) -> Result<Vec<Message>, DBError> {
  use crate::database::schema::messages;
  Ok(
    messages::table
      .filter(messages::id.eq_any(message_ids))
      .select(Message::as_select())
      .get_results::<Message>(conn)
      .map_err(|err| {
        tracing::error!(
          "Failed to get messages from ids {:?}: {}",
          message_ids,
          err.to_string()
        );
        return DBError::QueryError("Failed to get message".into());
      })?,
  )
}

pub fn update_message(
  conn: &mut PoolPGConnectionType,
  message_id: i32,
  update_data: UpdateMessage,
) -> Result<Message, DBError> {
  use crate::database::schema::messages;
  let mut updated_at_datetime = None;
  if update_data.content.is_some() || update_data.message_type.is_some() {
    updated_at_datetime = Some(Utc::now().naive_utc());
  }
  let message = diesel::update(messages::table.find(message_id))
    .set((
      update_data
        .content
        .map(|content| messages::content.eq(content)),
      update_data
        .message_type
        .map(|mt| messages::message_type.eq(mt)),
      updated_at_datetime.map(|datetime| messages::updated_at.eq(datetime)),
    ))
    .returning(Message::as_returning())
    .get_result::<Message>(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to update message {}: {}",
        message_id,
        err.to_string()
      );
      return DBError::QueryError("Failed to delete message".into());
    })?;
  Ok(message)
}

pub fn delete_messages(
  conn: &mut PoolPGConnectionType,
  message_ids: &Vec<i32>,
) -> Result<bool, DBError> {
  let result = diesel::delete(messages::table)
    .filter(messages::id.eq_any(message_ids))
    .execute(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to delete message with ids: {:?}, cause: {}",
        &message_ids,
        err.to_string()
      );
      DBError::QueryError("Failed to delete messages".to_string());
    });
  if result.unwrap() > 0 {
    Ok(true)
  } else {
    Ok(false)
  }
}
pub fn check_owner_of_messages(
  conn: &mut PoolPGConnectionType,
  user_id: i32,
  message_ids: &Vec<i32>,
) -> Result<Vec<i32>, diesel::result::Error> {
  let rs = messages::table
    .filter(
      messages::id
        .eq_any(message_ids)
        .and(messages::user_id.ne(user_id)),
    )
    .select(messages::id)
    .get_results::<i32>(conn)?;
  Ok(rs)
}

pub fn change_messages_status(
  conn: &mut PoolPGConnectionType,
  message_ids: &Vec<i32>,
  status: MessageStatus,
) -> Result<(), DBError> {
  diesel::update(messages::table)
    .filter(messages::id.eq_any(message_ids))
    .set(messages::status.eq(status))
    .execute(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to change status of messages ids {:?}: {}",
        message_ids,
        err.to_string()
      );
      return DBError::QueryError("Failed to change status of messages".into());
    })?;
  Ok(())
}
