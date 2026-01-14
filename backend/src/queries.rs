
use actix_web::web;
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    models::{Invitation, Pool, User},
    schema::users as users_table,
    errors::ServiceError,
};

pub async fn get_user_by_email_query(
    email: &str,
    pool: web::Data<Pool>,
) -> Result<Option<User>, ServiceError> {
    
    let mut conn = pool.get().unwrap();

    users_table::dsl::users
        .filter(users_table::email.eq(email.to_lowercase()))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional()
        .map_err(|_| ServiceError::BadRequest("Error checking user by id".to_string()))
}

pub async fn get_user_by_oidc_subject_query(
    oidc_subject: &str,
    pool: web::Data<Pool>,
) -> Result<User, ServiceError> {
    let mut conn = pool.get().map_err(|_e| {
        ServiceError::InternalServerError("Failed to get postgres connection".to_owned())
    })?;

    let user = users_table::dsl::users
        .filter(users_table::dsl::oidc_subject.eq(oidc_subject))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .map_err(|_| {
            ServiceError::BadRequest(
                "Error loading user by itself for get_user_by_oidc_subject_query".to_owned(),
            )
        })?;

    Ok(user)
}

pub async fn associate_user_to_oidc_subject_query(
    email: &str,
    oidc_subject: String,
    pool: web::Data<Pool>,
) -> Result<(), ServiceError> {
    let mut conn = pool.get().map_err(|_e| {
        ServiceError::InternalServerError("Failed to get postgres connection".to_string())
    })?;

    diesel::update(users_table::dsl::users.filter(users_table::email.eq(email.to_lowercase())))
        .set(users_table::dsl::oidc_subject.eq(oidc_subject))
        .execute(&mut conn)
        .map_err(|_| ServiceError::BadRequest("Error updating user".to_string()))?;

    Ok(())
}

pub async fn create_user_query(
    user_oidc_subject: Option<String>,
    email: String,
    pool: web::Data<Pool>,
) -> Result<User, ServiceError> {
    let mut conn = pool.get().map_err(|_e| {
        ServiceError::InternalServerError("Failed to get postgres connection".to_string())
    })?;

    let user = User::from_details_with_oidc_subject(user_oidc_subject, email);
    let user = conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            let user = diesel::insert_into(users_table::dsl::users)
                .values(&user)
                .get_result::<User>(conn)?;

            Ok(user)
        })
        .map_err(|err| {
            ServiceError::InternalServerError(format!("Failed to create user {:?}", err))
        })?;

    Ok(user)
}