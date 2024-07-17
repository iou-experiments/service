use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub has_double_spent: Option<bool>,
  pub nonce: Option<String>,
  pub username: Option<String>,
  pub pubkey: Option<String>,
  pub messages: Option<Vec<String>>,
  pub notes: Option<Vec<String>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserSchema {
    pub username: String,
    pub pubkey: String,
    pub nonce: String,
    pub messages: Vec<String>,
    pub notes: Vec<String>,
    pub has_double_spent: bool,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UsernameRequest {
  pub username: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NoteSchema {
  recipient: String,
  sender: String,
  message: String,
  timestamp: i64,
  attachment: NoteNullifierSchema,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NoteHistorySchema {
  note: NoteSchema,
  history: Vec<NoteSchema>,
  spent: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageSchema {
  pub recipient: String,
  pub sender: String,
  pub message: String,
  pub timestamp: i64,
  pub attachment_id: String,
  pub read: bool, 
  pub _id: Option<Bson>
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MessageRequestSchema {
  pub recipient: String,
  pub sender: String,
  pub message: String,
  pub attachment_id: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct NoteNullifierSchema {
  pub nullifier: String,
  pub note: String, // Note structure serialized as JSON
  pub step: i32,
  pub owner: String, // Address serialized as JSON
}
#[derive(Debug, Deserialize, Serialize)]
pub struct NullifierRequest {
  pub nullifier: String,
}

