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
  nullifier: String,
  user_pub_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageSchema {
  recipient: String,
  sender: String,
  message: String,
  timestamp: i64,
  attachment: NoteNullifierSchema,
}
