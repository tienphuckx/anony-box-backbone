use diesel::{
  dsl::count, BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
  SelectableHelper,
};

use crate::{
  database::{
    models::{Group, WaitingList},
    schema::{groups, participants, waiting_list},
  },
  errors::DBError,
  PoolPGConnectionType,
};

pub fn check_user_join_group(
  conn: &mut PoolPGConnectionType,
  user_id: i32,
  group_id: i32,
) -> Result<bool, DBError> {
  use crate::database::schema::participants;
  let count = participants::table
    .filter(
      participants::user_id
        .eq(user_id)
        .and(participants::group_id.eq(group_id)),
    )
    .count()
    .get_result::<i64>(conn)
    .map_err(|err| {
      tracing::error!("database err: {}", err.to_string());
      DBError::QueryError("Failed to check user joining group".into())
    })?;
  return if count > 0 { Ok(true) } else { Ok(false) };
}

pub fn get_count_waiting_list(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
) -> Result<i64, DBError> {
  use crate::database::schema::waiting_list;
  let count = waiting_list::table
    .filter(waiting_list::group_id.eq(group_id))
    .count()
    .get_result::<i64>(conn)
    .map_err(|err| {
      tracing::error!(
        "Failed to count waiting members for group_id {}: {:?}",
        group_id,
        err
      );
      DBError::QueryError(format!("Error counting waiting members: {:?}", err))
    })?;
  Ok(count as i64)
}

pub fn check_owner_of_group(
  conn: &mut PoolPGConnectionType,
  user_id: i32,
  group_id: i32,
) -> Result<bool, diesel::result::Error> {
  use crate::database::schema::groups;
  let count = groups::table
    .filter(groups::id.eq(group_id).and(groups::user_id.eq(user_id)))
    .count()
    .get_result::<i64>(conn)?;
  Ok(if count > 0 { true } else { false })
}

pub fn get_waiting_list_object(
  conn: &mut PoolPGConnectionType,
  request_id: i32,
) -> Result<Option<WaitingList>, diesel::result::Error> {
  use crate::database::schema::waiting_list;
  waiting_list::table
    .filter(waiting_list::id.eq(request_id))
    .select(WaitingList::as_select())
    .get_result::<WaitingList>(conn)
    .optional()
}

pub fn process_joining_request(
  conn: &mut PoolPGConnectionType,
  request: WaitingList,
  is_approved: bool,
) -> Result<(), diesel::result::Error> {
  let _ =
    diesel::delete(waiting_list::table.filter(waiting_list::id.eq(request.id))).execute(conn)?;
  if is_approved {
    let new_participant = (
      participants::group_id.eq(request.group_id),
      participants::user_id.eq(request.user_id),
    );
    diesel::insert_into(participants::table)
      .values(new_participant)
      .execute(conn)?;
  }
  Ok(())
}

pub fn get_group_info(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
) -> Result<Option<Group>, DBError> {
  Ok(
    groups::table
      .find(group_id)
      .select(Group::as_select())
      .first::<Group>(conn)
      .optional()
      .map_err(|err| {
        tracing::error!(
          "Failed to get_group_info from group_id {}: {:?}",
          group_id,
          err
        );
        DBError::QueryError(format!("Error counting joined members: {:?}", err))
      })?,
  )
}

pub fn get_count_participants(
  conn: &mut PoolPGConnectionType,
  group_id: i32,
) -> Result<i64, DBError> {
  Ok(
    participants::table
      .filter(participants::group_id.eq(group_id))
      .select(count(participants::user_id))
      .first::<i64>(conn)
      .map_err(|err| {
        tracing::error!(
          "Failed to count joined members for group_id {}: {:?}",
          group_id,
          err
        );
        DBError::QueryError(format!("Error counting joined members: {:?}", err))
      })?,
  )
}
