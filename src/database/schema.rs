// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "attachmenttype"))]
    pub struct Attachmenttype;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "messagestatustype"))]
    pub struct Messagestatustype;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "messagetype"))]
    pub struct Messagetype;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Attachmenttype;

    attachments (id) {
        id -> Int4,
        #[max_length = 255]
        url -> Varchar,
        attachment_type -> Attachmenttype,
        message_id -> Int4,
    }
}

diesel::table! {
    groups (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        group_code -> Varchar,
        user_id -> Int4,
        approval_require -> Nullable<Bool>,
        maximum_members -> Nullable<Int4>,
        created_at -> Nullable<Timestamp>,
        expired_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Messagetype;
    use super::sql_types::Messagestatustype;

    messages (id) {
        id -> Int4,
        #[max_length = 1000]
        content -> Nullable<Varchar>,
        message_type -> Messagetype,
        created_at -> Timestamp,
        user_id -> Int4,
        group_id -> Int4,
        message_uuid -> Uuid,
        updated_at -> Nullable<Timestamp>,
        status -> Messagestatustype,
    }
}

diesel::table! {
    participants (user_id, group_id) {
        user_id -> Int4,
        group_id -> Int4,
        id -> Int4,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 255]
        user_code -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    waiting_list (user_id, group_id) {
        user_id -> Int4,
        group_id -> Int4,
        #[max_length = 1000]
        message -> Nullable<Varchar>,
        created_at -> Timestamp,
        id -> Int4,
    }
}

diesel::joinable!(attachments -> messages (message_id));
diesel::joinable!(groups -> users (user_id));
diesel::joinable!(messages -> groups (group_id));
diesel::joinable!(messages -> users (user_id));
diesel::joinable!(participants -> groups (group_id));
diesel::joinable!(participants -> users (user_id));
diesel::joinable!(waiting_list -> groups (group_id));
diesel::joinable!(waiting_list -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    attachments,
    groups,
    messages,
    participants,
    users,
    waiting_list,
);
