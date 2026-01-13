use actix_web::{HttpResponse, web};
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    errors::ServiceError,
    models::{Invitation, Pool, SlimUser, User},
    utils::hash_password,
};

// UserData is used to extract data from a post request by the client
#[derive(Debug, Deserialize)]
pub struct UserData {
    pub password: String,
}

/// Not used (keycloak user registration is used instead)
pub async fn register_user(
    invitation_id: web::Path<String>,
    user_data: web::Json<UserData>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user = web::block(move || {
        create_user_by_inv_query(
            invitation_id.into_inner(),
            user_data.into_inner().password,
            pool,
        )
    })
    .await??;

    Ok(HttpResponse::Ok().json(&user))
}

fn create_user_by_inv_query(
    invitation_id: String,
    password: String,
    pool: web::Data<Pool>,
) -> Result<SlimUser, crate::errors::ServiceError> {
    use crate::schema::{invitations as invitations_table, users as users_table};

    let mut conn = pool.get().unwrap();

    let invitation_id = invitation_id.parse::<Uuid>()?;

    invitations_table::dsl::invitations
        .filter(invitations_table::dsl::id.eq(invitation_id))
        .load::<Invitation>(&mut conn)
        .map_err(|_db_error| ServiceError::BadRequest("Invalid Invitation".into()))
        .and_then(|mut result| {
            if let Some(invitation) = result.pop() {
                // if invitation is not expired
                if invitation.expires_at > chrono::Local::now().naive_local() {
                    // try hashing the password, else return the error that will be converted to ServiceError
                    let hash: String = hash_password(&password)?;
                    dbg!(&hash);

                    let user = User::from_details_with_hash(invitation.email, hash);
                    let inserted_user: User = diesel::insert_into(users_table::dsl::users)
                        .values(&user)
                        .get_result(&mut conn)?;
                    dbg!(&inserted_user);

                    return Ok(inserted_user.into());
                }
            }
            Err(ServiceError::BadRequest("Invalid Invitation".into()))
        })
}
