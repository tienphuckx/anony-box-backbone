use std::{env, sync::Arc, time::Duration};

use axum::{
  routing::{any, get, post},
  Router,
};
use axum::http::{HeaderValue, Method};
use dotenvy::dotenv;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tower_http::cors::{CorsLayer, Any};

use crate::{
  handlers,
  payloads::{
    common::{CommonResponse, ListResponse},
    groups::{GroupInfo,
             GroupListResponse,
             NewGroupForm,
             WaitingListResponse,
             DelGroupResponse,
             DelGroupRequest,
             GrDetailSettingResponse,
             RmUserRequest,
             RmUserResponse},
    user::{NewUserRequest, UserResponse},
  },
  AppState,
};
use crate::handlers::group::{get_gr_setting, get_gr_setting_v1};

#[derive(OpenApi)]
#[openapi(
  paths(
    handlers::common::home,
    handlers::group::get_list_groups_by_user_id,
    handlers::group::create_user_and_group,
    handlers::group::join_group,
    handlers::group::get_waiting_list,
    handlers::group::process_joining_request,
    handlers::group::del_gr_req,
    handlers::group::get_gr_setting,
    handlers::group::get_gr_setting_v1,
    handlers::group::rm_user_from_gr,
    handlers::group::user_leave_gr,
    handlers::user::add_user_docs
    
  ),
  components(schemas(
    NewGroupForm, NewUserRequest,
    UserResponse, CommonResponse<UserResponse>,
    GroupListResponse, GroupInfo,
    ListResponse<WaitingListResponse>,
    DelGroupRequest, DelGroupResponse,
    GrDetailSettingResponse,
    RmUserRequest, RmUserResponse
  ))
)]
struct ApiDoc;

pub fn get_swagger_ui() -> SwaggerUi {
  SwaggerUi::new("/swagger-ui").url("/api/docs/open-api.json", ApiDoc::openapi())
}

pub fn init_router() -> Router<Arc<AppState>> {

  // Load environment variables from .env file
  dotenv().ok();

  // Get WEB_CLIENT from environment variables
  let web_client_origin = env::var("WEB_CLIENT")
      .expect("WEB_CLIENT must be set in .env")
      .parse::<HeaderValue>()
      .expect("Invalid WEB_CLIENT URL");

  // Configure CORS to allow requests from the web client
  let cors = CorsLayer::new()
      .allow_origin(web_client_origin)
      .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
      .allow_headers(Any);

  Router::new()

    .route("/", get(handlers::common::home))
    .route("/del-gr", post(handlers::group::del_gr_req))
    .route("/rm-rf-group", post(handlers::group::rm_rf_group))
    .route("/rm-u-from-gr", post(handlers::group::rm_user_from_gr))
    .route("/leave-gr", post(handlers::group::user_leave_gr))
    .route("/add-user-group",post(handlers::group::create_user_and_group))
    .route("/v1/add-user-group",post(handlers::group::create_user_and_group_v1))
    .route("/join-group", post(handlers::group::join_group))
    .route("/gr/list/:user_id", get(handlers::group::get_list_groups_by_user_id))
    .route("/groups/:group_id/waiting-list", get(handlers::group::get_waiting_list))
    .route("/waiting-list/:request_id", post(handlers::group::process_joining_request))
    .route("/add-user", post(handlers::user::add_user)) //first: create a new user
    .route("/create-group",post(handlers::group::create_group_with_user))
    .route("/send-msg", post(handlers::message::send_msg))
    .route("/group-detail/:group_id", get(handlers::message::get_group_detail_with_extra_info))
    .route("/group-detail/setting/:gr_id/:u_id", get(get_gr_setting))
    .route("/group-detail/setting/:gr_id", get(get_gr_setting_v1))
    .route("/get-latest-messages/:group_code",get(handlers::message::get_latest_messages_by_code))
    .route("/add-user-doc", post(handlers::user::add_user_docs))
    .route("/ws", any(handlers::socket::handler::ws_handler))

    .fallback(handlers::common::fallback)
    .merge(get_swagger_ui())
    .layer(TraceLayer::new_for_http())
    .layer(cors)
    .layer(TimeoutLayer::new(Duration::from_secs(10)))
}
