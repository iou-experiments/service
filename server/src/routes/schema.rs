use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserSchema {
    pub username: String,
    pub pubkey: String,
    pub nonce: String,
    pub messages: Vec<String>,
    pub notes: Vec<String>,
    pub has_double_spent: bool,
    pub id: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct NoteNullifierSchema {
  _id: ObjectId,
  nullifier: String,
  user_pub_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageSchema {
  pub recipient: String,
  pub sender: String,
  pub message: String,
  pub timestamp: i64,
  pub attachment_id: String,
  pub read: bool, 
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MessageRequestSchema {
  pub recipient: String,
  pub sender: String,
  pub message: String,
  pub attachment_id: String,
}