use axum::{extract::Extension, http::StatusCode, Json, response::IntoResponse};
use crate::mongo::IOUServiceDB;
use super::{
  schema::{AuthData, CreateUserSchema, UserRequest, UserIdentifier},
  error::{DatabaseError, ErrorResponse}
};
use mongodb::bson::doc;
use uuid::Uuid;

#[axum::debug_handler]
pub async fn get_user(
    Extension(db): Extension<IOUServiceDB>,
    Json(payload): Json<UserRequest>
) -> impl IntoResponse {
    let result = match payload.identifier {
        UserIdentifier::Username(username) => db.get_user_with_username(&username).await,
        UserIdentifier::Address(address) => db.get_user_with_address(&address).await,
    };

    match result {
        Ok(user_response) => {
            (StatusCode::OK, Json(user_response.user)).into_response()
        },
        Err(err) => {
            let (status, error_message) = match err.current_context() {
                DatabaseError::NotFoundError => (
                    StatusCode::NOT_FOUND,
                    format!("User not found: {}", err)
                ),
                DatabaseError::FetchError => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to fetch user: {}", err)
                ),
                DatabaseError::ConversionError => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to process user data: {}", err)
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("An unexpected error occurred: {}", err)
                ),
            };
            (status, Json(ErrorResponse { error: error_message })).into_response()
        }
    }
}

#[axum::debug_handler]
pub async fn create_user(
    Extension(db): Extension<IOUServiceDB>,
    Json(payload): Json<CreateUserSchema>
) -> impl IntoResponse {
  match db.create_user(&payload).await {
    Ok(user_response) => {
      println!("{:#?}", user_response);
      (StatusCode::CREATED, Json(user_response)).into_response()
    },
    Err(err) => {
      let error_message = format!("Failed to create user: {}", err);
      (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: error_message })).into_response()
    }
  }
}

#[axum::debug_handler]
pub async fn create_and_send_challenge(
  Extension(state): Extension<IOUServiceDB>,
  Json(username): Json<String>,
) -> Result<Json<String>, (StatusCode, Json<String>)> {
  match state.get_challenge(None, &username).await {
    Ok(challenge) => Ok(Json(hex::encode(challenge))),
    Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json("Failed to create challenge".to_string()))),
  }
}

pub async fn verify_challenge(
  Extension(state): Extension<IOUServiceDB>,
  Json(auth_request): Json<AuthData>,
) -> impl IntoResponse {
  match state.authenticate_user(&auth_request.username, &auth_request.signature_hex, &auth_request.challenge_id).await {
    Ok(true) => {
      // Create a session token
      let session_id = Uuid::new_v4().to_string();
      state.insert_session(session_id.clone(), auth_request.username.clone());

      Ok((StatusCode::OK, Json(session_id)))
    },
    Ok(false) => Ok((StatusCode::UNAUTHORIZED, Json("Invalid signature".to_string()))),
    Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed".to_string())),
  }
}

pub async fn validate_session(
  Extension(state): Extension<IOUServiceDB>,
  Json(session_id): Json<String>,
) -> Result<String, (StatusCode, String)> {
  let sessions = state.sessions.read().unwrap();

  if sessions.get(&session_id).is_some() {
    Ok("authenticated".to_owned())
  } else {
      Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()))
  }
}