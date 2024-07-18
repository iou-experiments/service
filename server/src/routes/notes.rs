use axum::{extract::Extension, http::StatusCode, Json};
use crate::mongo::IOUServiceDB;
use crate::routes::schema::NoteSchema;
use super::{response::NoteResponse, schema::NoteRequest};
use mongodb::bson::doc;

#[axum::debug_handler]
pub async fn get_notes(
    Extension(db): Extension<IOUServiceDB>,
    Json(payload): Json<NoteRequest>
) -> Result<Json<Vec<NoteSchema>>, String> {

    let note_response = db.get_user_notes(&payload.owner_pub_key).await;
    Ok(Json(note_response.unwrap()))
}

#[axum::debug_handler]
pub async fn save_note(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<NoteSchema>) -> Result<Json<NoteResponse>, StatusCode> {
  let new_note = NoteSchema {
    owner: payload.owner,
    asset_hash: payload.asset_hash,
    value: payload.value,
    step: payload.step,
    parent_note: payload.parent_note,
    out_index: payload.out_index,
    blind: payload.blind
  };
  println!("{:#?}", new_note);
  match db.store_note(&new_note).await {
    Ok(note_response) => {
      println!("{:#?}", note_response);
      Ok(Json(note_response))
    }
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}
