use std::future::{Ready, ready};

use actix_identity::Identity;
use actix_session::Session;
use actix_web::{
    Either, Error, FromRequest, HttpMessage as _, HttpRequest, HttpResponse, dev::Payload, http::{StatusCode, header::{self, ContentType}}, web
};
use diesel::prelude::*;

use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse,
};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{AccessTokenHash, ClientId, IssuerUrl, Nonce, ProviderMetadataWithLogout};
use serde::{Deserialize, Serialize};

use crate::{
    app_utils::build_authorization_cookie, 
    endpoint::login_success_json_response, errors::ServiceError, 
    get_env, 
    models::{Pool, SlimUser, User},
    queries::{associate_user_to_oidc_subject_query, create_user_query, get_user_by_email_query, get_user_by_oidc_subject_query},
    utils::{SECRET_KEY, verify_password},
};

#[derive(Debug, Deserialize)]
pub struct AuthData {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthQuery {
    /// URL to redirect to after successful login
    pub redirect_uri: Option<String>,
    /// Code sent via email as a result of successful call to send_invitation
    pub inv_code: Option<uuid::Uuid>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginState {
    /// URL to redirect to after successful login
    pub redirect_uri: String,
    /// Code sent via email as a result of successful call to send_invitation
    pub inv_code: Option<uuid::Uuid>,
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

#[derive(Deserialize, Debug)]
pub struct OpCallback {
    pub state: String,
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenIdConnectState {
    pub pkce_verifier: PkceCodeVerifier,
    pub csrf_token: CsrfToken,
    pub nonce: Nonce,
}

const OIDC_SESSION_KEY: &str = "oidc_state";

pub async fn logout(id: Identity) -> HttpResponse {
    id.logout();
    HttpResponse::NoContent().finish()
}

pub async fn login_plain_text(
    req: HttpRequest,
    body: Either<web::Json<AuthData>, web::Form<AuthData>>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, actix_web::Error> {

    log::info!("login_plain_text() entered");
    let auth_data  = match body {
        Either::Left(json) => json.into_inner(),
        Either::Right(form) => form.into_inner(),
    };
    let user = web::block(move || query_auth(auth_data, pool)).await??;

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
fn query_auth(auth_data: AuthData, pool: web::Data<Pool>) -> Result<SlimUser, ServiceError> {
    use crate::schema::users::dsl::{email, users};

    let mut conn = pool.get().unwrap();

    let mut items = users
        .filter(email.eq(&auth_data.email))
        .load::<User>(&mut conn)?;

    if let Some(user) = items.pop() {
        if let Some(ref hash) = user.hash {
            if let Ok(matching) = verify_password(hash, &auth_data.password) {
                if matching {
                    return Ok(user.into());
                }
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

pub async fn build_oidc_client() -> CoreClient {
    let issuer_url = get_env!(
        "OIDC_ISSUER_URL",
        "Issuer URL for OpenID provider must be set"
    )
    .to_string();

    let client_id = get_env!(
        "OIDC_CLIENT_ID",
        "Client ID for OpenID provider must be set"
    )
    .to_string();

    let auth_redirect_url = get_env!(
        "OIDC_AUTH_REDIRECT_URL",
        "Auth redirect URL for OpenID provider must be set"
    )
    .to_string();
    let client_secret = get_env!(
        "OIDC_CLIENT_SECRET",
        "Client secret for OpenID provider must be set"
    )
    .to_string();
    let base_server_url = get_env!(
        "BASE_SERVER_URL",
        "Server hostname for OpenID provider must be set"
    );

    //build OpenId Connect client
    let meta_data = CoreProviderMetadata::discover_async(
        IssuerUrl::new(issuer_url.clone()).expect("IssuerUrl for OpenID provider must be set"),
        async_http_client,
    )
    .await
    .map_err(|err| format!("Failed to discover OIDC provider {:?}", err))
    .unwrap();

    CoreClient::new(
        ClientId::new(client_id.clone()),
        Some(ClientSecret::new(client_secret.clone())),
        IssuerUrl::new(issuer_url.clone()).expect("IssuerUrl for OpenID provider must be set"),
        AuthUrl::new(auth_redirect_url.clone()).expect("Auth configuration is not a valid URL"),
        meta_data.token_endpoint().cloned(),
        meta_data.userinfo_endpoint().cloned(),
        meta_data.jwks().to_owned(),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/api/auth/callback", base_server_url))
            .expect("Redirect URL for OpenID provider must be set"),
    )
}

pub async fn login_via_provider(
    req: HttpRequest,
    session: Session,
    data: web::Query<AuthQuery>,
    oidc_client: web::Data<CoreClient>,
) -> Result<HttpResponse, Error> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    log::info!("login_via_provider enterred");

    let (auth_url, csrf_token, nonce) = oidc_client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scopes([
            Scope::new("profile".to_owned()),
            Scope::new("email".to_owned()),
        ])
        .set_pkce_challenge(pkce_challenge)
        .url();

    let oidc_state = OpenIdConnectState {
        pkce_verifier,
        csrf_token,
        nonce,
    };

    session
        .insert(OIDC_SESSION_KEY, oidc_state)
        .map_err(|err| {
            ServiceError::InternalServerError(format!("Could not set OIDC Session {:?}", err))
        })?;

    let redirect_uri = match data.redirect_uri.clone() {
        Some(redirect_uri) => redirect_uri,
        None => req
            .headers()
            .get("Referer")
            .map(|h| h.to_str().unwrap_or("/"))
            .unwrap_or("/")
            .to_string(),
    };

    let login_state = LoginState {
        redirect_uri,
        inv_code: data.inv_code,
    };

    session.insert("login_state", login_state).map_err(|err| {
        ServiceError::InternalServerError(format!("Could not set redirect url {:?}", err))
    })?;

    //redirect to OpenIdProvider for authentication
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", auth_url.as_str()))
        .finish())
}

pub async fn oidc_callback_test(
    req: HttpRequest,
    session: Session,
    //oidc_client: web::Data<CoreClient>,
    //pool: web::Data<Pool>,
    //query: web::Query<OpCallback>,
) -> Result<HttpResponse, Error> {
    log::info!("oidc_callback entered");
    Ok(HttpResponse::NoContent().finish())
}


pub async fn oidc_callback(
    req: HttpRequest,
    session: Session,
    oidc_client: web::Data<CoreClient>,
    pool: web::Data<Pool>,
    query: web::Query<OpCallback>,
) -> Result<HttpResponse, Error> {
    log::info!("oidc_callback entered, query: {:?}", query);
    let state: OpenIdConnectState = session
        .get(OIDC_SESSION_KEY)
        .map_err(|e| {
            log::info!("oidc_callback get(OIDC_SESSION_KEY) error: {}", e.to_string());
            ServiceError::InternalServerError("Could not get OIDC Session".into())
        })?
        .ok_or(ServiceError::Unauthorized)?;

    let code_verifier = state.pkce_verifier;
    let code = query.code.clone();
    let nonce = state.nonce;

    let token_response = oidc_client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(code_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            log::info!("oidc_callback oidc_client::request_async error: {}", e.to_string());
            match e {
            oauth2::RequestTokenError::ServerResponse(e) => {
                ServiceError::InternalServerError(e.to_string())
            }
            oauth2::RequestTokenError::Request(e) => {
                ServiceError::InternalServerError(e.to_string())
            }
            oauth2::RequestTokenError::Parse(e, _) => {
                ServiceError::InternalServerError(e.to_string())
            }
            oauth2::RequestTokenError::Other(e) => ServiceError::InternalServerError(e.to_string()),
        }})?;

    let id_token = token_response
        .extra_fields()
        .id_token()
        .ok_or_else(|| ServiceError::InternalServerError("Empty ID Token".into()))?;

    let id_token_verifier = oidc_client.id_token_verifier();
    let claims = id_token.claims(&id_token_verifier, &nonce).map_err(|e| {
        ServiceError::InternalServerError(format!("Claims Verification Error, {}", e))
    })?;

    match claims.access_token_hash() {
        None => {
            log::warn!("Access Token Hash Not provided by openid provider, skipping hash check");
            Ok(())
        }
        Some(given_token_hash) => {
            let calculated_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                &id_token.signing_alg().map_err(|_| {
                    ServiceError::BadRequest("ID token hash unavailable".to_string())
                })?,
            )
            .map_err(|_| ServiceError::BadRequest("ID token hash unavailable".to_string()))?;

            if calculated_token_hash != *given_token_hash {
                Err(ServiceError::BadRequest(
                    "ID token hash invalid".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }?;

    let user_oidc_subject = claims.subject().to_string();

    let email = claims.email().ok_or_else(|| {
        ServiceError::InternalServerError("Failed to parse email from claims".into())
    })?;

    /*let name = claims.name().ok_or_else(|| {
        ServiceError::InternalServerError("Failed to parse name from claims".into())
    })?;*/

    let login_state = session
        .get::<LoginState>("login_state")
        .map_err(|_| ServiceError::InternalServerError("Could not get redirect url".into()))?
        .ok_or(ServiceError::Unauthorized)?;

    let mut user_is_new = false;

    // Check if a user with this email exists
    let optional_user_from_email = get_user_by_email_query(email, pool.clone()).await?;

    let user_from_oidc_subject =
        get_user_by_oidc_subject_query(&user_oidc_subject, pool.clone()).await;

    let user = match (user_from_oidc_subject, &optional_user_from_email) {
        (Ok(user), _) => user,
        (Err(_), Some(User {
            oidc_subject: None, ..
        })) => {
            // User exists, but has no current oidc subject in postgres need to associate them
            log::info!("calling associate_user_to_oidc_subject_query for user_oidc_subject={user_oidc_subject}");
            associate_user_to_oidc_subject_query(email, user_oidc_subject.clone(), pool.clone())
                .await?;

            get_user_by_oidc_subject_query(&user_oidc_subject, pool.clone()).await?
        },
        (Err(_), None) => {
            // User does not exist from oidc_subject or have a matching email
            user_is_new = true;
            log::info!("calling create_user_query for user_oidc_subject={user_oidc_subject}");
            let user = create_user_query(
                Some(user_oidc_subject),
                email.to_string(),
                pool.clone(),
            )
            .await?;
            user
        },
        (
            Err(_),
            Some(User {
                oidc_subject: Some(_),
                ..
            }),
        ) => {
            // This should not be reachable, we should error out.
            // Something fishy is going on with our auth provider.
            unreachable!();
        }
    };

    let user_string = serde_json::to_string(&user).map_err(|_| {
        ServiceError::InternalServerError("Failed to serialize user to JSON".into())
    })?;

    /*let mut redis_conn = redis_pool
        .get()
        .await
        .map_err(|err| ServiceError::BadRequest(err.to_string()))?;*/

    /*let slim_user = SlimUser::from(user);
    let slim_user_string = serde_json::to_string(&slim_user).map_err(|_| {
        ServiceError::InternalServerError("Failed to serialize slim user to JSON".into())
    })?;*/

    /*redis_conn
        .set::<_, _, ()>(slim_user.id.to_string(), slim_user_string)
        .await
        .map_err(|err| ServiceError::BadRequest(err.to_string()))?;*/

    Identity::login(&req.extensions(), user_string).expect("Failed to set login state for user");
    session.remove(OIDC_SESSION_KEY);
    session.remove("login_state");

    // Add a query param if the user has just been created and is the owner of
    // one organization
    let mut final_redirect = login_state.redirect_uri.clone();
    if user_is_new {
        // Add query param indicating new user
        if final_redirect.contains('?') {
            final_redirect = format!("{}&new_user=true", final_redirect);
        } else {
            final_redirect = format!("{}?new_user=true", final_redirect);
        }
    }

    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", final_redirect))
        .finish())
}