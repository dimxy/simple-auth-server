//! App utils.
//! Used this repo: https://github.com/behai-nguyen/rust_web_01.git // TODO: not used yet (only examples code)

#![allow(unused_imports)]
#![allow(dead_code)]

use actix_web::{
    http::{header, StatusCode, header::ContentType}, HttpRequest, cookie::{Cookie, SameSite}, 
    HttpResponse, Responder, body::BoxBody, //HttpMessage
};

pub static REDIRECT_MESSAGE: &str = "redirect-message";
pub static ORIGINAL_CONTENT_TYPE: &str = "original-content-type";

pub static TOKEN_TYPE: &str = "bearer";
pub static BEARER_TOKEN: &str = "Bearer.";

pub fn build_cookie<'a>(
    name: &'a str,
    value: &'a str,
    server_only: bool,
    removal: bool
) -> Cookie<'a> {
    let mut cookie = Cookie::build(name, value)
        .path("/")
        .secure(true)
        .http_only(server_only)
        .same_site(SameSite::None)
        .finish();

    if removal {
        cookie.make_removal();
    }

    cookie
}

pub fn build_login_redirect_cookie<'a> (
    value: &'a str
) -> Cookie<'a> {
    build_cookie(REDIRECT_MESSAGE, value, true, false)
}


pub fn remove_login_redirect_cookie<'a>() -> Cookie<'a> {
    build_cookie(REDIRECT_MESSAGE, "", true, true)
}

pub fn build_authorization_cookie<'a>(
    access_token: &'a str
) -> Cookie<'a> {
    build_cookie(header::AUTHORIZATION.as_str(), access_token, false, false)
}

pub fn remove_authorization_cookie<'a>() -> Cookie<'a> {
    build_cookie(header::AUTHORIZATION.as_str(), "", false, true)
}