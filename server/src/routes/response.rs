use crate::routes::schema::User;
use serde::Serialize;
use crate::routes::schema::{MessageSchema, NoteNullifierSchema};

#[derive(Debug, Serialize)]
pub struct UserSingleResponse {
    pub status: &'static str,
    pub user: User,
}

#[derive(Debug, Serialize)]
pub struct MessageSingleResponse {
    pub status: &'static str,
    pub message: MessageSchema,
}
#[derive(Debug, Serialize)]
pub struct NullifierResponseData {
    pub status: &'static str,
    pub nullifier: NoteNullifierSchema,
}
#[derive(Debug, Serialize)]
pub enum NullifierResponse {
  Ok(NoteNullifierSchema),
  NotFound,
  Error,
}