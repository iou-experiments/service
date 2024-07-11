use axum::http::Error;
use bson::{doc, oid, Array, Document};
use mongodb::{ options::{ FindOneOptions, ClientOptions, ServerApi, ServerApiVersion }, Client, Collection };
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
  #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
  pub id: Option<i128>,
  pub hasDoubleSpent: Option<bool>,
  pub nonce: Option<i128>,
  pub username: Option<String>,
  pub pubkey: Option<String>,
  pub messages: Option<Vec<()>>,
  pub notes: Option<Vec<()>>
}

pub struct IOUServiceDB {
  pub users: Collection<User>,
  pub note_history: Collection<()>,
  pub  messages: Collection<()>,
  pub nullifiers: Collection<()>
}

impl IOUServiceDB {
  pub async fn init() -> Self {
    let uri = env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
    println!("{}", uri);
    let mut client_options = ClientOptions::parse(uri).await.unwrap();
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("iou");
    let users = db.collection("users");

    let note_history = db.collection("note_history");
    let messages = db.collection("messages");
    let nullifiers = db.collection("nullifiers");

    Self {
      users,
      note_history,
      messages,
      nullifiers
    }
  }

  pub async fn create_user(&self, user: User) -> Result<crate::mongo::oid::ObjectId, &str> {
    // check if the username exists already in the database
    let query = doc! { "username": &user.username };
    let options = FindOneOptions::builder()
        .projection(doc! {"_id": 1})
        .build();
    match self.users.find_one(query).with_options(options).await.unwrap() {
        Some(_) => return Err("User exists"),
        None => (),
    };

    // insert the user into the collection
    match self.users.insert_one(&user).await {
        Ok(result) => Ok(result.inserted_id.as_object_id().unwrap()),
        Err(e) => Err("Err"),
    }
}


  pub async fn get_user(&self, username: &str) -> std::option::Option<User> {
    self.users
      .find_one(doc! { "username": "sero" })
      .await
      .unwrap()
  }
}