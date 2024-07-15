use ark_crypto_primitives::Error;

use bson::{doc, Document};
use mongodb::{ options::{  ClientOptions, ServerApi, ServerApiVersion }, Client, Collection};
use serde::{Deserialize, Serialize};
use crate::routes::{response::{MessageSingleResponse, UserSingleResponse}, schema::{CreateUserSchema, MessageRequestSchema}};
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use std::env;
use crate::routes::schema::{NoteSchema, NoteHistorySchema, MessageSchema, NoteNullifierSchema};
use chrono::Utc;

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

#[derive(Debug, Clone)]
pub struct IOUServiceDB {
  pub users_collection: Collection<User>,
  pub users: Collection<Document>,
  pub notes: Collection<Document>,
  pub notes_collection: Collection<NoteSchema>,
  pub note_history: Collection<Document>,
  pub note_history_collection: Collection<NoteHistorySchema>,
  pub messages: Collection<Document>,
  pub messages_collection: Collection<MessageSchema>,
  pub nullifiers: Collection<Document>,
  pub nullifiers_collection: Collection<NoteNullifierSchema>,

}

impl IOUServiceDB {
  pub async fn init() -> Self {
    let uri = env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
    let mut client_options = ClientOptions::parse(uri).await.unwrap();
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    let client = Client::with_options(client_options).unwrap();

    let db = client.database("iou");
    let users = db.collection::<Document>("users");
    let users_collection = db.collection("users");
    let notes = db.collection::<Document>("notes");
    let notes_collection = db.collection("notes");
    let note_history = db.collection::<Document>("note_history");
    let note_history_collection = db.collection("note_history");
    let messages = db.collection::<Document>("messages");
    let messages_collection = db.collection("messages");
    let nullifiers = db.collection::<Document>("nullifiers");
    let nullifiers_collection = db.collection("nullifiers");

    Self {
      users,
      users_collection,
      notes,
      notes_collection,
      note_history,
      note_history_collection,
      messages,
      messages_collection,
      nullifiers,
      nullifiers_collection,
    }
  }

  pub async fn get_user(&self, username: &str) -> UserSingleResponse{
    let user = self
    .users
    .find_one(doc! {"username": username}, None)
    .await;

    self.doc_to_user(user.unwrap().unwrap())
  }

  pub async fn create_user(&self, body: &CreateUserSchema) -> Result<UserSingleResponse, Error> {
    let document = self.create_user_document(body);

    let options: IndexOptions = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder()
        .keys(doc! {"title": 5})
        .options(options)
        .build();
    let res = match self.users.create_index(index, None).await {
        Ok(_) => {}
        Err(e) => return Err(Error::from(e)),
    };
    println!("{:#?}", res);
    let insert_result = match self.users.insert_one(document, None).await {
        Ok(result) => result,
        Err(e) => {
            if e.to_string()
                .contains("E11000 duplicate key error collection")
            {
              return Err(Error::from(e));
            }
            return Err(Error::from(e));
        }
    };
    let new_id = insert_result
        .inserted_id
        .as_object_id()
        .expect("issue with new _id");

    let user_doc = match self
        .users
        .find_one(doc! {"_id": new_id}, None)
        .await
    {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(Error::from("User not found after insertion")),
        Err(e) => return Err(Error::from(e))
    };

    Ok(UserSingleResponse {
        status: "success",
        user: self.doc_to_user(user_doc).user
    })
}

  fn doc_to_user(&self, doc: Document) -> UserSingleResponse {
    let user = User {
      id: doc.get_str("_id").ok().map(|s| s.to_owned()),
        has_double_spent: doc.get_bool("has_double_spent").ok(),
        nonce: doc.get_str("nonce").ok().map(|s| s.to_owned()),
        username: doc.get_str("username").ok().map(|s| s.to_owned()),
        pubkey: doc.get_str("pubkey").ok().map(|s| s.to_owned()),
        messages: doc.get_array("messages").ok().map(|arr| 
            arr.iter().filter_map(|bson| bson.as_str().map(|s| s.to_owned())).collect()),
        notes: doc.get_array("notes").ok().map(|arr| 
            arr.iter().filter_map(|bson| bson.as_str().map(|s| s.to_owned())).collect()),
    };

    UserSingleResponse {
      status: "success",
      user
    }
  }

  fn doc_to_message(&self, doc: Document) -> MessageSingleResponse {
    let message = MessageSchema {
      sender: doc.get_str("sender").ok().map(|s| s.to_owned()).unwrap(),
      recipient: doc.get_str("recipient").ok().map(|s| s.to_owned()).unwrap() ,
      message: doc.get_str("message").ok().map(|s| s.to_owned()).unwrap(),
      timestamp: doc.get_i64("timestamp").ok().unwrap(),
      attachment_id: doc.get_str("attachment_id").ok().map(|s| s.to_owned()).unwrap(),
      read: doc.get_bool("read").ok().unwrap(),
    };

    MessageSingleResponse {
      status: "success",
      message: message
    }
  }

  fn create_user_document(&self, body: &CreateUserSchema) -> Document {
    let user = doc! {
      "username": body.username.clone(),
      "pubkey": body.pubkey.clone(),
      "nonce": body.nonce.clone(),
      "messages": body.messages.clone(),
      "notes": body.notes.clone(),
      "has_double_spent": body.has_double_spent
    };
    
    user
  }
  
  fn get_current_timestamp(&self) -> i64 {
    Utc::now().timestamp()
  }

  fn create_message_document(&self, body: &MessageRequestSchema) -> Document {
    let message = doc! {
      "sender": body.sender.clone(),
      "recipient": body.recipient.clone(),
      "message": body.message.clone(),
      "timestamp": self.get_current_timestamp(),
      "attachment_id": body.attachment_id.clone(),
      "read": false,
    };

    message
  }

  pub async fn send_message(&self, body: &MessageRequestSchema) -> Result<MessageSingleResponse, Error> {
    let document = self.create_message_document(body);
    let options: IndexOptions = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder()
        .keys(doc! {"title": 5})
        .options(options)
        .build();
    let res = match self.users.create_index(index, None).await {
        Ok(_) => {}
        Err(e) => return Err(Error::from(e)),
    };
    let insert_result = match self.messages.insert_one(document, None).await {
        Ok(result) => result,
        Err(e) => {
            if e.to_string()
                .contains("E11000 duplicate key error collection")
            {
              return Err(Error::from(e));
            }
            return Err(Error::from(e));
        }
    };
    println!("{:#?}", res);
    let new_id = insert_result
        .inserted_id
        .as_object_id()
        .expect("issue with new _id");

    let message_doc = match self
        .messages
        .find_one(doc! {"_id": new_id}, None)
        .await
    {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(Error::from("User not found after insertion")),
        Err(e) => return Err(Error::from(e))
    };

    Ok(self.doc_to_message(message_doc))
  }

}