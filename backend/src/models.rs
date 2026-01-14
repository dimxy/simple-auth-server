//! Data models
//! Partially used code from this repo: https://github.com/behai-nguyen/rust_web_01.git // TODO: not used yet (only examples code)

#![allow(clippy::unused)]
#![allow(clippy::extra_unused_lifetimes)]

use chrono::{NaiveDateTime, TimeDelta, Utc};
use diesel::{self, PgConnection, r2d2::ConnectionManager, Queryable, Insertable};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::api_status::ApiStatus;

use super::schema::*;

// type alias to use in multiple places
pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Serialize, Deserialize, Selectable, Queryable, Insertable)]
#[diesel(table_name = users)]
pub struct User {
    pub email: String,
    pub hash: Option<String>,
    pub created_at: NaiveDateTime,
    pub oidc_subject: Option<String>,
}

impl User {
    pub fn from_details_with_hash<S: Into<String>>(email: S, hash: String) -> Self {
        User {
            oidc_subject: None,
            email: email.into(),
            hash: Some(hash),
            created_at: chrono::Utc::now().naive_local(),
        }
    }

    pub fn from_details_with_oidc_subject<S: Into<String>, T: Into<String>>(
        oidc_subject: Option<T>,
        email: S,
    ) -> Self {
        User {
            oidc_subject: oidc_subject.map(|s| s.into()),
            email: email.into(),
            hash: None,
            created_at: chrono::Utc::now().naive_local(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Selectable, Queryable, Insertable)]
#[diesel(table_name = invitations)]
pub struct Invitation {
    pub id: Uuid,
    pub email: String,
    pub expires_at: NaiveDateTime,
}

// any type that implements Into<String> can be used to create Invitation
impl<T> From<T> for Invitation
where
    T: Into<String>,
{
    fn from(email: T) -> Self {
        Invitation {
            id: Uuid::new_v4(),
            email: email.into(),
            expires_at: (Utc::now() + TimeDelta::try_hours(24).unwrap()).naive_utc(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlimUser {
    pub email: String,
}

impl From<User> for SlimUser {
    fn from(user: User) -> Self {
        SlimUser { email: user.email }
    }
}

/// Represents a result data of a successful login request.
/// **Work in progress**. 
/// 
#[derive(Serialize, Deserialize, Debug)]
pub struct LoginSuccess {
    /// Logged in user email.
    pub email: String,
    /// The login authentication token.
    pub access_token: String,
    pub token_type: String,
}

/// Represents a JSON response of a successful login request.
#[derive(Serialize, Deserialize)]
pub struct LoginSuccessResponse {
    #[serde(flatten)]
    /// **Work in progress**. All API responses should have this data.
    pub api_status: ApiStatus,
    /// The actual data component of the response.
    pub data: LoginSuccess
}
