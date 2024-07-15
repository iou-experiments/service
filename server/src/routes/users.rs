use axum::{extract::Extension, http::StatusCode, Json};
use crate::mongo::{IOUServiceDB, User};
use super::{response::UserSingleResponse, schema::{CreateUserSchema, UsernameRequest}};
use mongodb::bson::doc;

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
  println!("{}", "i am inside");
  let new_user = CreateUserSchema {
    username: payload.username,
    pubkey: payload.pubkey,
    nonce: payload.nonce,
    messages: payload.messages,
    notes: payload.notes,
    has_double_spent: payload.has_double_spent,
    id: payload.id,
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