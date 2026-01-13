//#[macro_use]
//extern crate diesel;
#![allow(unused_imports)]
#![allow(dead_code)]

use crate::auth_handler::build_oidc_client;
use actix_identity::IdentityMiddleware;
use actix_session::{Session, SessionMiddleware, config::PersistentSession, storage::CookieSessionStore};
use actix_web::{
    App, HttpServer, HttpRequest, HttpResponse, cookie::Key, middleware, error, get, web, Result,
    http::{
        Method, StatusCode,
        header::{self, ContentType},
    },
};
use actix_files::Files;
use actix_cors::Cors;
use diesel::{prelude::*, r2d2};
use time::Duration;

mod auth_handler;
mod auth_middleware;
mod email_service;
mod email_service_ssmtp;
mod errors;
mod invitation_handler;
mod models;
mod register_handler;
mod schema;
mod utils;
mod app_utils;
mod api_status;
mod endpoint;
//mod jwt_utils;
mod messages;
mod queries;

const BIND_PORT: u16 = 8080;

/// simple index handler
#[get("/welcome")]
async fn welcome(req: HttpRequest, session: Session) -> Result<HttpResponse> {
    println!("{req:?}");

    // session
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {count}");
        counter = count + 1;
    }

    // set counter to session
    session.insert("counter", counter)?;

    // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type(ContentType::plaintext())
        .body(include_str!("../static/index.html")))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // create db connection pool
    let manager = r2d2::ConnectionManager::<PgConnection>::new(database_url);
    let pool: models::Pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let domain: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_owned());

    log::info!("Connecting to OIDC");
    let oidc_client = build_oidc_client().await;

    log::info!("starting HTTP server at http://localhost:{BIND_PORT}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(oidc_client.clone()))
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    Key::from(utils::SECRET_KEY.as_bytes()),
                )
                .session_lifecycle(PersistentSession::default().session_ttl(Duration::days(1)))
                .cookie_name("auth-example".to_owned())
                .cookie_secure(false)
                .cookie_domain(Some(domain.clone()))
                //.cookie_http_only(true)
                .cookie_path("/".to_owned())
                .build(),
            )
            // enable logger
            .wrap(middleware::Logger::default())
            //.wrap(Cors::permissive())
            //.wrap(auth_middleware::CheckLogin)
            //.service(welcome)
            // everything under '/api/' route
            .service(
                web::scope("/api")
                    .service(
                        web::resource("/invitation")
                            .route(web::post().to(invitation_handler::post_invitation)),
                    )
                    .service(
                        web::resource("/register/{invitation_id}")
                            .route(web::post().to(register_handler::register_user)),
                    )
                    .service(
                        web::scope("/auth")
                            .service(
                                web::resource("")
                                    .route(web::post().to(auth_handler::login_via_provider))
                                    .route(web::get().to(auth_handler::login_via_provider))
                                    .route(web::delete().to(auth_handler::logout))
                            )
                            .service(
                                web::resource("me")
                                    .route(web::get().to(auth_handler::get_me)),
                            )
                            .service(
                                web::resource("/callback")
                                    .route(web::get().to(auth_handler::oidc_callback)),
                            )
                    )
                    .service(
                        web::resource("/whoami")
                            .route(web::get().to(auth_handler::whoami)),
                    ),
            )
            .service(Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(("127.0.0.1", BIND_PORT))?
    .run()
    .await
}
