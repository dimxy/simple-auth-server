//! Application request handler helper methods.
//! Used this repo: https://github.com/behai-nguyen/rust_web_01.git // TODO: not used yet (only examples code)

#![allow(unused_imports)]
#![allow(dead_code)]

use actix_web::http::StatusCode;

use serde_json;

use crate::models::{LoginSuccess, LoginSuccessResponse};
use crate::api_status::ApiStatus;
use crate::app_utils::TOKEN_TYPE;

pub fn http_status_code(status_code: StatusCode) -> u16 {
    status_code.as_u16()
}

pub fn serialise_api_status(
    status_code: StatusCode,
    message: &str,
    session_id: Option<String>
) -> String {
    let mut api_status = ApiStatus::new(http_status_code(status_code))
        .set_message(message);

    if let Some(sess_id) = session_id {
        api_status = api_status.set_session_id(&sess_id);
    }

    serde_json::to_string(&api_status).unwrap()
}

pub fn login_success_json_response(
    email: &str, 
    access_token: &str) -> String {

    let r = LoginSuccessResponse {
        api_status: ApiStatus::new(http_status_code(StatusCode::OK)),
        data: LoginSuccess { email: String::from(email), 
            access_token: String::from(access_token),
            token_type: String::from(TOKEN_TYPE)
        }
    };

    serde_json::to_string(&r).unwrap()
}