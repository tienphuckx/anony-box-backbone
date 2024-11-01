use diesel::{
  r2d2::ConnectionManager, BoolExpressionMethods, ExpressionMethods, OptionalExtension,
  PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use r2d2::PooledConnection;

use crate::database::{
  models::WaitingList,
  schema::{participants, waiting_list},
};

pub fn check_user_join_group(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  user_id: i32,
  group_id: i32,
) -> Result<bool, diesel::result::Error> {
  use crate::database::schema::participants;
  let count = participants::table
    .filter(
      participants::user_id
        .eq(user_id)
        .and(participants::group_id.eq(group_id)),
    )
    .count()
    .get_result::<i64>(conn)?;
  return if count > 0 { Ok(true) } else { Ok(false) };
}

pub fn get_count_waiting_list(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
  group_id: i32,
) -> Result<i64, diesel::result::Error> {
  use crate::database::schema::waiting_list;
  let count = waiting_list::table
    .filter(waiting_list::group_id.eq(group_id))
    .count()
    .get_result::<i64>(conn)?;
  Ok(count as i64)
}

pub fn check_owner_of_group(
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
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
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
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
  conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
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
