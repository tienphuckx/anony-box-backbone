use std::{env, sync::Arc, time::Duration};

use axum::{
  extract::DefaultBodyLimit, routing::{any, delete, get, post}, Router
};
use axum::http::{HeaderValue, Method};
use dotenvy::dotenv;
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tower_http::cors::{CorsLayer, Any};

use crate::{
  handlers,
  payloads::{
    common::{OrderBy, CommonResponse, ListResponse},
    groups::*, messages::*, user::{NewUserRequest, UserResponse}
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
    handlers::group::del_gr_req,
    handlers::group::get_gr_setting_v1,
    handlers::group::rm_user_from_gr,
    handlers::group::user_leave_gr,
    handlers::group::get_group_detail_with_extra_info, 
    handlers::message::send_msg,
    handlers::message::get_messages,
    handlers::message::update_message,
    handlers::message::delete_message,
    handlers::user::add_user_docs,
    handlers::file::upload_file,
    handlers::file::serve_file
    
  ),
  components(schemas(
    OrderBy,
    NewGroupForm, NewUserRequest,
    UserResponse, CommonResponse<UserResponse>,
    GroupListResponse, GroupInfo,
    ListResponse<WaitingListResponse>,
    DelGroupRequest, DelGroupResponse,
    GrDetailSettingResponse, 
    SendMessageRequest, SendMessageResponse,
    AttachmentPayload,
    MessageResponse,
    ListResponse<MessageWithUser>,
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
    .route("/messages", post(handlers::message::send_msg))
    .route("/messages/:message_id", delete(handlers::message::delete_message).put(handlers::message::update_message))
    .route("/groups/:group_id/messages", get(handlers::message::get_messages))
    .route("/group-detail/:group_id", get(handlers::group::get_group_detail_with_extra_info))
    .route("/group-detail/setting/:gr_id", get(handlers::group::get_gr_setting_v1))
    .route("/add-user-doc", post(handlers::user::add_user_docs))
    .route("/files", post(handlers::file::upload_file))
    .route("/files/:filename", get(handlers::file::serve_file))
    .route("/ws", any(handlers::socket::handler::ws_handler))
    .fallback(handlers::common::fallback)
    .merge(get_swagger_ui())
    .layer(TraceLayer::new_for_http())
    .layer(cors)
    .layer(TimeoutLayer::new(Duration::from_secs(10)))
    .layer(DefaultBodyLimit::disable())
    .layer(RequestBodyLimitLayer::new(10* 1024 * 1024))
}
