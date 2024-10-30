use std::sync::Arc;

use axum::{
  routing::{get, post},
  Router,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
  handlers,
  payloads::{
    common::CommonResponse,
    groups::{GroupInfo, GroupListResponse, NewGroupForm},
    user::{NewUserRequest, UserResponse},
  },
  AppState,
};

#[derive(OpenApi)]
#[openapi(
  paths(
    handlers::common::home,
    handlers::group::get_user_groups,
    handlers::group::create_user_and_group,
    handlers::group::join_group,
    handlers::user::add_user_docs
    
  ),
  components(schemas(
    NewGroupForm, NewUserRequest,
    UserResponse, CommonResponse<UserResponse>,
    GroupListResponse, GroupInfo
  ))
)]
struct ApiDoc;

pub fn get_swagger_ui() -> SwaggerUi {
  SwaggerUi::new("/swagger-ui").url("/api/docs/open-api.json", ApiDoc::openapi())
}

pub fn init_router() -> Router<Arc<AppState>> {
  Router::new()
    .route("/", get(handlers::common::home))
    .route(
      "/add-user-group",
      post(handlers::group::create_user_and_group),
    )
    .route("/join-group", post(handlers::group::join_group))
    .route("/gr/list/:user_id", get(handlers::group::get_user_groups))
    .route("/add-user", post(handlers::user::add_user)) //first: create a new user
    .route(
      "/create-group",
      post(handlers::group::create_group_with_user),
    )
    .route("/send-msg", post(handlers::message::send_msg))
    .route(
      "/get-latest-messages",
      post(handlers::message::get_latest_messages),
    )
    .route(
      "/get-latest-messages/:group_code",
      get(handlers::message::get_latest_messages_by_code),
    )
    .route("/add-user-doc", post(handlers::user::add_user_docs))
    .merge(get_swagger_ui())
}
