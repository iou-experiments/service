use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserSchema {
    pub username: String,
    pub pubkey: String,
    pub nonce: String,
    pub messages: Vec<String>,
    pub notes: Vec<String>,
    pub hasDoubleSpent: bool,
    pub id: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UsernameRequest {
  pub username: String
}
#[derive(Serialize, Deserialize, Debug)]
pub struct NotesSchema {

}
#[derive(Serialize, Deserialize, Debug)]
pub struct MessageSchema {

}
