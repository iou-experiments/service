use axum::{extract::Extension, http::StatusCode, Json, response::IntoResponse};
use crate::mongo::IOUServiceDB;
use super::{response::NullifierResponse, schema::{NoteNullifierSchema,  NullifierRequest}};
use mongodb::bson::doc;
use super::error::ErrorResponse;

#[axum::debug_handler]
pub async fn verify_nullifier(
  Extension(db): Extension<IOUServiceDB>,
  Json(payload): Json<NullifierRequest>
) -> Result<Json<NullifierResponse>, String> {
  let nullifier_response = db.get_nullifier(&payload.nullifier, &payload.state).await;
  Ok(Json(nullifier_response))
}

#[axum::debug_handler]
pub async fn store_nullifier(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<NoteNullifierSchema>) -> impl IntoResponse {
  let new_nullifier = NoteNullifierSchema {
    nullifier: payload.nullifier,
    note: payload.note,
    step: payload.step,
    owner: payload.owner,
    state: payload.state, 
  };
  println!("{:#?}", new_nullifier);
  match db.store_nullifier(&new_nullifier).await {
    Ok(nullifier_res) => {
      println!("{:#?}", nullifier_res);
      (StatusCode::OK, Json(nullifier_res)).into_response()
    },
    Err(err) => {
      let nullifier_res_err = format!("Failed to get send message: {}", err);
      (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: nullifier_res_err })).into_response()
    }
  }
}