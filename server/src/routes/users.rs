use axum::{extract::Extension, http::StatusCode, Json, response::IntoResponse};
use crate::mongo::IOUServiceDB;
use crate::routes::schema::User;
use super::{response::UserSingleResponse, schema::{AuthData, CreateUserSchema, UsernameRequest}};
use mongodb::bson::doc;
use uuid::Uuid;

#[axum::debug_handler]
pub async fn get_user_with_username(
    Extension(db): Extension<IOUServiceDB>,
    Json(payload): Json<UsernameRequest>
) -> Result<Json<User>, String> {

    let user_response = db.get_user(&payload.username).await;
    Ok(Json(user_response.user))
}

#[axum::debug_handler]
pub async fn create_user(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<CreateUserSchema>) -> Result<Json<UserSingleResponse>, StatusCode> {
  let new_user = CreateUserSchema {
    username: payload.username,
    pubkey: payload.pubkey,
    nonce: payload.nonce,
    messages: payload.messages,
    notes: payload.notes,
    has_double_spent: payload.has_double_spent,
  };
  println!("{:#?}", new_user);
  match db.create_user(&new_user).await {
    Ok(user_response) => {
      println!("{:#?}", user_response);
      Ok(Json(user_response))
    }
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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

async fn validate_session(
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