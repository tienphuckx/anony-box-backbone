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
