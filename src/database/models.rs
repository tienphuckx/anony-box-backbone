use chrono::NaiveDateTime;
use diesel::{
  prelude::{Associations, Identifiable, Insertable, Queryable},
  Selectable,
};

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


// Define the MessageText struct for the messages_text table
#[derive(Queryable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = crate::database::schema::messages_text)]
pub struct MessageText {
  pub id: i32,
  pub content: Option<String>,        // Nullable content in the database
  pub message_type: String,           // Stored as Varchar
  pub created_at: NaiveDateTime,
  pub user_id: i32,
  pub group_id: i32,
}

// Define NewMessageText struct for inserting new messages into messages_text table
#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::messages_text)]
pub struct NewMessageText<'a> {
  pub content: Option<&'a str>,       // Content can be nullable
  pub message_type: &'a str,          // Store type as Varchar directly
  pub created_at: NaiveDateTime,
  pub user_id: i32,
  pub group_id: i32,
}