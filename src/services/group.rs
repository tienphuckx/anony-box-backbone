use diesel::{
  r2d2::ConnectionManager, BoolExpressionMethods, ExpressionMethods, PgConnection, QueryDsl,
  RunQueryDsl,
};
use r2d2::PooledConnection;

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
