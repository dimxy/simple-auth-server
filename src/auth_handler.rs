use std::future::{Ready, ready};

use actix_identity::Identity;
use actix_web::{
    Either, Error, FromRequest, HttpMessage as _, HttpRequest, HttpResponse, dev::Payload, http::{StatusCode, header::{self, ContentType}}, web
};
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    app_utils::build_authorization_cookie, endpoint::login_success_json_response, errors::ServiceError,
    //jwt_utils::{make_token, make_bearer_token},
    models::{Pool, SlimUser, User}, utils::{SECRET_KEY, verify}
};

#[derive(Debug, Deserialize)]
pub struct AuthData {
    pub email: String,
    pub password: String,
}

// we need the same data
// simple aliasing makes the intentions clear and its more readable
pub type LoggedUser = SlimUser;

impl FromRequest for LoggedUser {
    type Error = Error;
    type Future = Ready<Result<LoggedUser, Error>>;

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        if let Ok(identity) = Identity::from_request(req, pl).into_inner() {
            if let Ok(user_json) = identity.id() {
                if let Ok(user) = serde_json::from_str(&user_json) {
                    return ready(Ok(user));
                }
            }
        }

        ready(Err(ServiceError::Unauthorized.into()))
    }
}

pub async fn logout(id: Identity) -> HttpResponse {
    id.logout();
    HttpResponse::NoContent().finish()
}

pub async fn login(
    req: HttpRequest,
    body: Either<web::Json<AuthData>, web::Form<AuthData>>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, actix_web::Error> {

    log::info!("login() entered");
    let auth_data  = match body {
        Either::Left(json) => json.into_inner(),
        Either::Right(form) => form.into_inner(),
    };
    let user = web::block(move || query(auth_data, pool)).await??;

    let user_string = serde_json::to_string(&user).unwrap();
    Identity::login(&req.extensions(), user_string).unwrap();

    //let access_token = make_token(&user_string, SECRET_KEY.as_ref(), 1 * 60);

    //Identity::login(&req.extensions(), String::from( make_bearer_token(&access_token) )).unwrap();

    Ok(HttpResponse::NoContent().finish())
    // The request content type is "application/x-www-form-urlencoded", returns the home page.
    /*if req.content_type() == ContentType::form_url_encoded().to_string() {
        Ok(HttpResponse::Ok()
            //.status(StatusCode::SEE_OTHER)
            // Note this header.
            .append_header((header::AUTHORIZATION, String::from(&access_token)))
            //.append_header((header::LOCATION, "/"))
            // Note this client-side cookie.
            .cookie(build_authorization_cookie(&access_token))
            .content_type(ContentType::html())
            .body(render_home_page(&req)
        ))
    }
    else {
        // The request content type is "application/json", returns a JSON content of
        // LoginSuccessResponse.
        // 
        // Token field is the access token which the users need to include in the future 
        // requests to get authenticated and hence access to protected resources.		
        Ok(HttpResponse::Ok()
            //.status(StatusCode::SEE_OTHER)
            // Note this header.
            .append_header((header::AUTHORIZATION, String::from(&access_token)))
            //.append_header((header::LOCATION, "/"))
            // Note this client-side cookie.
            .cookie(build_authorization_cookie(&access_token))
            .content_type(ContentType::json())
            .body(login_success_json_response(&user_string, &access_token)
        ))
    }*/
}

pub async fn get_me(logged_user: LoggedUser) -> HttpResponse {
    HttpResponse::Ok().json(logged_user)
}
/// Diesel query
fn query(auth_data: AuthData, pool: web::Data<Pool>) -> Result<SlimUser, ServiceError> {
    use crate::schema::users::dsl::{email, users};

    let mut conn = pool.get().unwrap();

    let mut items = users
        .filter(email.eq(&auth_data.email))
        .load::<User>(&mut conn)?;

    if let Some(user) = items.pop() {
        if let Ok(matching) = verify(&user.hash, &auth_data.password) {
            if matching {
                return Ok(user.into());
            }
        }
    }
    Err(ServiceError::Unauthorized)
}

/// Renders the home page and return the complete content as a 
/// [`std::string::String`].
fn render_home_page(_req: &HttpRequest) -> &'static str {
    // Create a new Tera instance and add a template from a string
    //let tera = Tera::new("templates/auth/**/*").unwrap();

    //let ctx = Context::new();

    //tera.render("home.html", &ctx).expect("Failed to render template")
    include_str!("../static/home.html")
}

pub async fn whoami(_logged_user: LoggedUser) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().json("ok"))
}