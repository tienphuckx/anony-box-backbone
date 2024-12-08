use std::io::Write;

use super::schema::sql_types::{Attachmenttype, Messagestatustype, Messagetype};
use chrono::NaiveDateTime;
use diesel::{
  deserialize::{self, FromSql, FromSqlRow},
  prelude::{Associations, Identifiable, Insertable, Queryable},
  serialize::{self, Output, ToSql},
  AsExpression, Selectable,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Selectable, Queryable, Identifiable)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
  pub id: i32,
  pub username: String,
  pub user_code: String,
  pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::users)]
pub struct NewUser<'a> {
  pub username: &'a str,
  pub user_code: &'a str,
  pub created_at: NaiveDateTime,
}

#[derive(Selectable, Queryable, Identifiable, Associations)]
#[diesel(table_name = crate::database::schema::groups)]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Group {
  pub id: i32,
  pub name: String,
  pub group_code: String,
  pub user_id: i32,
  pub approval_require: Option<bool>,
  pub maximum_members: Option<i32>,
  pub created_at: Option<NaiveDateTime>,
  pub expired_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::groups)]
pub struct NewGroup<'a> {
  pub name: &'a str,
  pub group_code: &'a str,
  pub user_id: i32,
  pub approval_require: Option<bool>,
  pub maximum_members: Option<i32>,
  pub created_at: NaiveDateTime,
  pub expired_at: NaiveDateTime,
}
#[allow(dead_code)]
#[derive(Selectable, Queryable, Associations)]
#[diesel(table_name = crate::database::schema::waiting_list)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WaitingList {
  pub id: i32,
  pub user_id: i32,
  pub group_id: i32,
  pub message: Option<String>,
  pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::waiting_list)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewWaitingList {
  pub user_id: i32,
  pub group_id: i32,
  pub message: Option<String>,
  pub created_at: NaiveDateTime,
}

#[derive(Selectable, Queryable, Associations, Insertable)]
#[diesel(table_name = crate::database::schema::participants)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Participant {
  pub id: i32,
  pub user_id: i32,
  pub group_id: i32,
}

// Custom Message type
#[derive(
  Debug, PartialEq, FromSqlRow, AsExpression, Eq, Clone, Serialize, Deserialize, ToSchema,
)]
#[diesel(sql_type = crate::database::schema::sql_types::Messagetype)]
pub enum MessageTypeEnum {
  TEXT,
  ATTACHMENT,
}
impl Default for MessageTypeEnum {
  fn default() -> Self {
    Self::TEXT
  }
}

impl ToSql<Messagetype, diesel::pg::Pg> for MessageTypeEnum {
  fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
    let status_str = match *self {
      MessageTypeEnum::TEXT => "TEXT",
      MessageTypeEnum::ATTACHMENT => "ATTACHMENT",
    };
    out.write_all(status_str.as_bytes())?;
    Ok(serialize::IsNull::No)
  }
}

impl FromSql<Messagetype, diesel::pg::Pg> for MessageTypeEnum {
  fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
    match bytes.as_bytes() {
      b"TEXT" => Ok(MessageTypeEnum::TEXT),
      b"ATTACHMENT" => Ok(MessageTypeEnum::ATTACHMENT),
      _ => Err("Unrecognized enum variant".into()),
    }
  }
}

#[derive(
  Debug, PartialEq, FromSqlRow, AsExpression, Eq, Clone, Serialize, Deserialize, ToSchema,
)]
#[diesel(sql_type = crate::database::schema::sql_types::Messagestatustype)]
pub enum MessageStatus {
  NotSent,
  Sent,
  Seen,
}
impl Default for MessageStatus {
  fn default() -> Self {
    Self::Sent
  }
}
impl ToSql<Messagestatustype, diesel::pg::Pg> for MessageStatus {
  fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
    let status_str = match *self {
      MessageStatus::NotSent => "NotSent",
      MessageStatus::Sent => "Sent",
      MessageStatus::Seen => "Seen",
    };
    out.write_all(status_str.as_bytes())?;
    Ok(serialize::IsNull::No)
  }
}

impl FromSql<Messagestatustype, diesel::pg::Pg> for MessageStatus {
  fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
    match bytes.as_bytes() {
      b"NotSent" => Ok(MessageStatus::NotSent),
      b"Sent" => Ok(MessageStatus::Sent),
      b"Seen" => Ok(MessageStatus::Seen),
      _ => Err("Unrecognized enum variant".into()),
    }
  }
}

// Custom AttachmentType type
#[derive(
  Debug, PartialEq, FromSqlRow, AsExpression, Eq, Serialize, Deserialize, ToSchema, Clone,
)]
#[diesel(sql_type = crate::database::schema::sql_types::Attachmenttype)]
pub enum AttachmentTypeEnum {
  TEXT,
  IMAGE,
  VIDEO,
  AUDIO,
  BINARY,
  COMPRESSION,
}
impl Default for AttachmentTypeEnum {
  fn default() -> Self {
    Self::TEXT
  }
}

impl AttachmentTypeEnum {
  pub fn default() -> Self {
    Self::TEXT
  }
}

impl ToSql<Attachmenttype, diesel::pg::Pg> for AttachmentTypeEnum {
  fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
    let status_str = match *self {
      AttachmentTypeEnum::TEXT => "TEXT",
      AttachmentTypeEnum::IMAGE => "IMAGE",
      AttachmentTypeEnum::VIDEO => "VIDEO",
      AttachmentTypeEnum::AUDIO => "AUDIO",
      AttachmentTypeEnum::BINARY => "BINARY",
      AttachmentTypeEnum::COMPRESSION => "COMPRESSION",
    };
    out.write_all(status_str.as_bytes())?;
    Ok(serialize::IsNull::No)
  }
}

impl FromSql<Attachmenttype, diesel::pg::Pg> for AttachmentTypeEnum {
  fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
    match bytes.as_bytes() {
      b"TEXT" => Ok(AttachmentTypeEnum::TEXT),
      b"IMAGE" => Ok(AttachmentTypeEnum::IMAGE),
      b"VIDEO" => Ok(AttachmentTypeEnum::VIDEO),
      b"AUDIO" => Ok(AttachmentTypeEnum::AUDIO),
      b"BINARY" => Ok(AttachmentTypeEnum::BINARY),
      b"COMPRESSION" => Ok(AttachmentTypeEnum::COMPRESSION),
      _ => Err("Unrecognized enum variant".into()),
    }
  }
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = crate::database::schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
  pub message_uuid: Uuid,
  pub id: i32,
  pub content: Option<String>,
  pub message_type: MessageTypeEnum,
  pub status: MessageStatus,
  pub created_at: NaiveDateTime,
  pub updated_at: Option<NaiveDateTime>,
  pub user_id: i32,
  pub group_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMessage<'a> {
  pub message_uuid: Uuid,
  pub content: Option<&'a String>,
  pub message_type: MessageTypeEnum,
  pub status: MessageStatus,
  pub created_at: NaiveDateTime,
  pub user_id: i32,
  pub group_id: i32,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(belongs_to(Message))]
#[diesel(table_name = crate::database::schema::attachments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Attachment {
  pub id: i32,
  pub url: String,
  pub attachment_type: AttachmentTypeEnum,
  pub message_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::attachments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAttachment<'a> {
  pub url: &'a str,
  pub message_id: i32,
  pub attachment_type: AttachmentTypeEnum,
}
