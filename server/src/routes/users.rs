use axum::{extract::Extension, Json};
use bson::Document;
use crate::mongo::{IOUServiceDB, User};
use mongodb::bson::doc;

#[axum::debug_handler]
pub async fn get_user_with_username(
    Extension(db): Extension<IOUServiceDB>,
) -> Result<Json<User>, String> {

    let user = db.get_user("sero").await;
    Ok(Json(user))
}

pub async fn create_user(Extension(db): Extension<IOUServiceDB>) -> Document {

  let new_doc = doc! {
    "username": "fred",
    "id": "2",
    "hasDoubleSpent": false,
    "nonce": "123",
    "pubkey": "123",
    "messages": ["123"],
    "notes": ["123"]
  };
  new_doc
  // db.create_user(new_doc);
}