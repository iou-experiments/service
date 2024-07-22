use axum::{body::Body, extract::Extension, http::StatusCode, middleware::Next, Json};
use crate::mongo::IOUServiceDB;
use crate::routes::schema::User;
use super::{response::UserSingleResponse, schema::{CreateUserSchema, UsernameRequest, AuthData}};
use mongodb::bson::doc;

use axum::{
  response::Response,
  extract::Request,
};

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

pub async fn auth_middleware<B>(
  Extension(db): Extension<IOUServiceDB>,
  AuthData { username, signature_hex, challenge_id }: AuthData,
  request: Request<Body>,
  next: Next,
) -> Result<Response, (StatusCode, String)> {
  let is_authenticated = db.authenticate_user(&username, &signature_hex, &challenge_id).await
      .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Authentication error: {}", e)))?;

  if is_authenticated {
      Ok(next.run(request).await)
  } else {
      Err((StatusCode::UNAUTHORIZED, "Authentication failed".to_string()))
  }
}