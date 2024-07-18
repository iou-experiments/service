use axum::{extract::Extension, http::StatusCode, Json};
use crate::mongo::IOUServiceDB;
use super::{response::NullifierResponse, response::NullifierResponseData, schema::{NoteNullifierSchema,  NullifierRequest}};
use mongodb::bson::doc;

#[axum::debug_handler]
pub async fn verify_nullifier(
  Extension(db): Extension<IOUServiceDB>,
  Json(payload): Json<NullifierRequest>
) -> Result<Json<NullifierResponse>, String> {
  let nullifier_response = db.get_nullifier(&payload.nullifier).await;
  Ok(Json(nullifier_response))
}

#[axum::debug_handler]
pub async fn store_nullifier(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<NoteNullifierSchema>) -> Result<Json<NullifierResponseData>, StatusCode> {
  let new_nullifier = NoteNullifierSchema {
    nullifier: payload.nullifier,
    note: payload.note,
    step: payload.step,
    owner: payload.owner,
  };
  println!("{:#?}", new_nullifier);
  match db.store_nullifier(&new_nullifier).await {
    Ok(nullifier_response) => {
      println!("{:#?}", nullifier_response);
      Ok(Json(nullifier_response))
    }
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}