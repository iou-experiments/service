use bson::{doc, Document};
use mongodb::{ options::{  ClientOptions, ServerApi, ServerApiVersion }, Client, Collection };
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub hasDoubleSpent: Option<bool>,
  pub nonce: Option<String>,
  pub username: Option<String>,
  pub pubkey: Option<String>,
  pub messages: Option<Vec<String>>,
  pub notes: Option<Vec<String>>
}

#[derive(Debug, Clone)]
pub struct IOUServiceDB {
  pub users: Collection<Document>,
  // pub note_history: Collection<()>,
  // pub  messages: Collection<()>,
  // pub nullifiers: Collection<()>
}

impl IOUServiceDB {
  pub async fn init() -> Self {
    let uri = env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
    let mut client_options = ClientOptions::parse(uri).await.unwrap();
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("iou");
    let users = db.collection("users");

    Self {
      users,
      // note_history,
      // messages,
      // nullifiers
    }
  }

  pub async fn get_user(&self, username: &str) -> User{
    let user = self
    .users
    .find_one(doc! {"username": username}, None)
    .await;

    self.doc_to_user(user.unwrap().unwrap())
  }
  

  fn doc_to_user(&self, doc: Document) -> User {
    let user_response = User {
      id: doc.get_str("_id").ok().map(|s| s.to_owned()),
        hasDoubleSpent: doc.get_bool("hasDoubleSpent").ok(),
        nonce: doc.get_str("nonce").ok().map(|s| s.to_owned()),
        username: doc.get_str("username").ok().map(|s| s.to_owned()),
        pubkey: doc.get_str("pubkey").ok().map(|s| s.to_owned()),
        messages: doc.get_array("messages").ok().map(|arr| 
            arr.iter().filter_map(|bson| bson.as_str().map(|s| s.to_owned())).collect()),
        notes: doc.get_array("notes").ok().map(|arr| 
            arr.iter().filter_map(|bson| bson.as_str().map(|s| s.to_owned())).collect()),
    };

    user_response
  }
}