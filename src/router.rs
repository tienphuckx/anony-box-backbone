use std::{sync::Arc, time::Duration};

use axum::{
  routing::{get, post},
  Router,
};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
  handlers,
  payloads::{
    common::{CommonResponse, ListResponse},
    groups::{GroupInfo, GroupListResponse, NewGroupForm, WaitingListResponse},
    user::{NewUserRequest, UserResponse},
  },
  AppState,
};

#[derive(OpenApi)]
#[openapi(
  paths(
    handlers::common::home,
    handlers::group::get_list_groups_by_user_id,
    handlers::group::create_user_and_group,
    handlers::group::join_group,
    handlers::group::get_waiting_list,
    handlers::group::process_joining_request,
    handlers::user::add_user_docs
    
  ),
  components(schemas(
    NewGroupForm, NewUserRequest,
    UserResponse, CommonResponse<UserResponse>,
    GroupListResponse, GroupInfo,
    ListResponse<WaitingListResponse>,
  ))
)]
struct ApiDoc;

pub fn get_swagger_ui() -> SwaggerUi {
  SwaggerUi::new("/swagger-ui").url("/api/docs/open-api.json", ApiDoc::openapi())
}

pub fn init_router() -> Router<Arc<AppState>> {
  Router::new()

    .route("/", get(handlers::common::home))

      .route("/del-gr", post(handlers::group::del_gr_req))
    .route(
      "/add-user-group",
      post(handlers::group::create_user_and_group),
    )
    .route("/join-group", post(handlers::group::join_group))
    .route("/gr/list/:user_id", get(handlers::group::get_list_groups_by_user_id))
    .route("/groups/:group_id/waiting-list", get(handlers::group::get_waiting_list))
    .route("/waiting-list/:request_id", post(handlers::group::process_joining_request))
    .route("/add-user", post(handlers::user::add_user)) //first: create a new user
    .route(
      "/create-group",
      post(handlers::group::create_group_with_user),
    )
    .route("/send-msg", post(handlers::message::send_msg))
      .route(
        "/group-detail/:group_id",
        get(handlers::message::get_group_detail_with_extra_info),
      )
    .route(
      "/get-latest-messages/:group_code",
      get(handlers::message::get_latest_messages_by_code),
    )
    .route("/add-user-doc", post(handlers::user::add_user_docs))

    .fallback(handlers::common::fallback)
    .merge(get_swagger_ui())
    .layer(
      (TraceLayer::new_for_http(),
      // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
      // requests don't hang forever.
      TimeoutLayer::new(Duration::from_secs(10)))
    )
}
