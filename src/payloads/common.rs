use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct CommonResponse<T> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

impl<T> CommonResponse<T> {
    pub fn success(data: T) -> Self {
        CommonResponse {
            code: 0,
            msg: "Success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        CommonResponse {
            code,
            msg: msg.to_string(),
            data: None,
        }
    }
}
