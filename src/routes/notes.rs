use axum::{extract::Extension, http::StatusCode, Json};
use crate::mongo::IOUServiceDB;
use crate::routes::schema::NoteSchema;
use super::{response::{MessageSingleResponse, NoteResponse}, schema::{
  NoteHistoryRequest, NoteHistorySaved, NoteRequest, SaveNoteRequestSchema, UsernameRequest
}};
use crate::routes::error::DatabaseError;
use mongodb::bson::doc;

#[axum::debug_handler]
pub async fn get_notes(
    Extension(db): Extension<IOUServiceDB>,
    Json(payload): Json<NoteRequest>
) -> Result<Json<Vec<NoteSchema>>, StatusCode> {
    let notes = db.get_user_notes(&payload.owner_pub_key).await
      .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filtered_notes = match payload.step {
      Some(step) => notes.into_iter().filter(|note| note.step == step).collect(),
      None => notes,
    };

    Ok(Json(filtered_notes))
}

#[axum::debug_handler]
pub async fn save_note(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<NoteSchema>) -> Result<Json<NoteResponse>, StatusCode> {
  let new_note = SaveNoteRequestSchema {
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

pub async fn create_and_transfer_note_history(
  Extension(db): Extension<IOUServiceDB>,
  Json(payload): Json<NoteHistoryRequest>
) -> Result<Json<MessageSingleResponse>, StatusCode> {
  // Call the database function
  let res: MessageSingleResponse = db.create_and_transfer_note_history(
      payload.owner_username,  // This converts Option<String> to Option<&str>
      &payload.recipient_username,
      payload.note_history,
      payload.message,
  )
  .await
  .map_err(|e| {
      eprintln!("Failed to transfer note: {:?}", e);
      StatusCode::INTERNAL_SERVER_ERROR
  })?;

  Ok(Json(res))
}

pub async fn get_user_note_history(
  Extension(db): Extension<IOUServiceDB>,
  Json(payload): Json<UsernameRequest>
) -> Result<Json<Vec<NoteHistorySaved>>, StatusCode> {
  match db.get_note_history_for_user(payload.username).await {
    Ok(notes) => {
        let response: Vec<NoteHistorySaved> = notes.into_iter()
            .map(|note| NoteHistorySaved::from(note))
            .collect();
        Ok(Json(response))
    },
    Err(e) => {
        eprintln!("Failed to get note history: {:?}", e);
        match e.current_context() {
            DatabaseError::NotFoundError => Err(StatusCode::NOT_FOUND),
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
}