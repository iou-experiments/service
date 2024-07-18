use ark_crypto_primitives::Error;

use bson::{doc, Document};
use mongodb::{ Cursor, options::{ ClientOptions, FindOptions, ServerApi, ServerApiVersion }, Client, Collection};
use crate::routes::{
  error::MyError,
  response::{
    MessageSingleResponse,
    NoteResponse,
    NullifierResponse,
    NullifierResponseData,
    UserSingleResponse
  },
  schema::{
    CreateUserSchema,
    MessageRequestSchema,
    User
  }
};
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use std::env;
use crate::routes::schema::{NoteSchema, MessageSchema, NoteNullifierSchema};
use chrono::Utc;
use futures::stream::TryStreamExt;

#[derive(Debug, Clone)]
pub struct IOUServiceDB {
  pub users_collection: Collection<User>,
  pub users: Collection<Document>,
  pub notes: Collection<Document>,
  pub notes_collection: Collection<NoteSchema>,
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
    // users
    let users = db.collection::<Document>("users");
    let users_collection = db.collection("users");
    // notes
    let notes = db.collection::<Document>("notes");
    let notes_collection = db.collection("notes");
    // communication
    let messages = db.collection::<Document>("messages");
    let messages_collection = db.collection("messages");
    // betrayal detection system
    let nullifiers = db.collection::<Document>("nullifiers");
    let nullifiers_collection = db.collection("nullifiers");

    Self {
      users,
      users_collection,
      notes,
      notes_collection,
      messages,
      messages_collection,
      nullifiers,
      nullifiers_collection,
    }
  }

  // User
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

  pub async fn get_user(&self, username: &str) -> UserSingleResponse {
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
        .keys(doc! {"username": 1})
        .options(options)
        .build();

    let res = match self.users.create_index(index, None).await {
        Ok(_) => {}
        Err(e) => return Err(Error::from(e)),
    };

    println!("{:#?}", res);
    let insert_result = match self.users.insert_one(document, None).await {
      Ok(result) => {
        println!("{:?}", result);
        result
      },
      Err(e) => {
        println!("{:?}", e);
          if e.to_string()
              .contains("E11000 duplicate key error collection")
          {
            return Err(Error::from(e));
          }
          return Err(Error::from(e));
      }
    };
    println!("{:#?}", insert_result);
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

  // Messages
  fn doc_to_message(&self, doc: Document) -> MessageSingleResponse {
    let message = MessageSchema {
      sender: doc.get_str("sender").ok().map(|s| s.to_owned()).unwrap(),
      recipient: doc.get_str("recipient").ok().map(|s| s.to_owned()).unwrap() ,
      message: doc.get_str("message").ok().map(|s| s.to_owned()).unwrap(),
      timestamp: doc.get_i64("timestamp").ok().unwrap(),
      attachment_id: doc.get_str("attachment_id").ok().map(|s| s.to_owned()).unwrap(),
      read: doc.get_bool("read").ok().unwrap(),
      _id: doc.get("_id").to_owned().cloned()
    };

    MessageSingleResponse {
      status: "success",
      message: message
    }
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
  
  fn get_current_timestamp(&self) -> i64 {
    Utc::now().timestamp()
  }

  pub async fn send_message(&self, body: &MessageRequestSchema) -> Result<MessageSingleResponse, Error> {
    let document = self.create_message_document(body);
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

    let recipient_username = message_doc.get_str("recipient")
    .map_err(|_| Error::from("Recipient username not found in message"))?;

    let update_result = self.users.update_one(
      doc! { "username": recipient_username }, 
      doc! { "$push": { "messages": new_id } },
      None,
    ).await;

    if let Err(err) = update_result {
      eprintln!("Error updating user document: {:?}", err); 
      return Err(Error::from("Failed to update user document with message"));
    }

    Ok(self.doc_to_message(message_doc))
  }

  pub async fn get_unread_messages(&self, username: &str) -> Result<Vec<MessageSchema>, Error> {
    let filter = doc! {
      "recipient": username,
      "read": false
    };

    let sort = doc! { "timestamp": 1 }; // 1 for ascending (oldest to newest)

    let find_options = FindOptions::builder()
      .sort(sort)
      .build();

    let mut cursor: Cursor<Document> = self.messages
        .find(filter, Some(find_options))
        .await
        .map_err(MyError::MongoError)?;
 
    let mut messages = Vec::new(); 

    while let Some(doc) = cursor.try_next().await? {
     
      let msg = self.doc_to_message(doc);

      let update_result = self.messages
        .update_one(
          doc! { "_id": msg.message._id.clone() }, 
          doc! { "$set": { "read": true } },
          None,
        )
        .await;

      if let Err(err) = update_result {
        eprintln!("Error marking message as read: {:?}", err);
        // Handle the error appropriately
      } else {
        messages.push(msg.message);
      }
    }

    Ok(messages)   
  }

  // Nullifiers
  fn doc_to_nullifier(&self, doc: Document) -> NullifierResponseData {
    let nullifier = NoteNullifierSchema {
      nullifier: doc.get_str("nullifier").ok().map(|s| s.to_owned()).unwrap(),
      note: doc.get_str("note").ok().map(|s| s.to_owned()).unwrap(),
      step: doc.get_i32("step").ok().unwrap(),
      owner: doc.get_str("owner").ok().map(|s| s.to_owned()).unwrap(),
    };

    NullifierResponseData {
      status: "success",
      nullifier
    }
  }

  fn create_note_nullifier_document(&self, body: &NoteNullifierSchema) -> Document {
    let nullifier = doc! {
      "nullifier": body.nullifier.clone(),
      "note": body.note.clone(),
      "step": body.step,
      "owner": body.owner.clone(),
    };

    nullifier
  }

  pub async fn store_nullifier(&self, body: &NoteNullifierSchema) -> Result<NullifierResponseData, Error> {
    let document = self.create_note_nullifier_document(body);
    let options: IndexOptions = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder()
        .keys(doc! {"nullifier": 2})
        .options(options)
        .build();
    let res = match self.nullifiers.create_index(index, None).await {
        Ok(_) => {}
        Err(e) => return Err(Error::from(e)),
    };
    let insert_result = match self.nullifiers.insert_one(document, None).await {
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
    let nullifier_doc = match self
      .nullifiers
      .find_one(doc! {"_id": new_id}, None)
      .await
    {
      Ok(Some(doc)) => doc,
      Ok(None) => return Err(Error::from("User not found after insertion")),
      Err(e) => return Err(Error::from(e))
    };
    
    Ok(self.doc_to_nullifier(nullifier_doc))
  }

  pub async fn get_nullifier(&self, nullifier: &str) -> NullifierResponse {
    let nullifier_doc = self
    .nullifiers
    .find_one(doc! {"nullifier": nullifier}, None)
    .await;

    match nullifier_doc {
      Ok(Some(doc)) => {
        // 1. Get the owner from the nullifier document
        let owner_result = doc.get_str("owner");

        if let Ok(owner) = owner_result {
            // 2. Update the user document
            let update_result = self.users
                .update_one(
                    doc! {"username": owner}, // Assuming username is used for identification
                    doc! {"$set": {"has_double_spent": true}},
                    None,
                )
                .await;

            // Handle potential errors during the update
            if let Err(err) = update_result {
                eprintln!("Error updating user: {:?}", err);
                // You might want to return an error response here as well
            }
        } else {
            eprintln!("Owner not found in nullifier document.");
        }
        println!("WARNING: USER IS ATTEMPTING TO DOUBLE SPEND, we have flagged their account.");
        NullifierResponse::Ok(self.doc_to_nullifier(doc).nullifier)
      }
      Ok(None) => NullifierResponse::NotFound, // Handle case where no document is found
      Err(err) => { 
          // Handle the error, e.g., log it
          println!("Error getting nullifier: {:?}", err);
          NullifierResponse::Error 
      }
    }
  } 

  // Notes
  fn doc_to_note(&self, doc: Document) -> NoteResponse {
    let note = NoteSchema {
      asset_hash: doc.get_str("asset_hash").ok().map(|s| s.to_owned()).unwrap(),
      owner: doc.get_str("owner").ok().map(|s| s.to_owned()).unwrap(),
      value: doc.get_i64("value").ok().unwrap() as u64,
      step: doc.get_i32("step").ok().unwrap() as u32,
      parent_note: doc.get_str("parent_note").ok().map(|s| s.to_owned()).unwrap(),
      out_index: doc.get_str("out_index").ok().map(|s| s.to_owned()).unwrap(),
      blind: doc.get_str("blind").ok().map(|s| s.to_owned()).unwrap(),
    };

    NoteResponse { status: "success", note }
  }

  fn create_note_document(&self, body: &NoteSchema) -> Document {
    let note = doc! {
      "asset_hash": body.asset_hash.clone(),
      "owner": body.owner.clone(),
      "value": body.value as i64,
      "step": body.step as i32,
      "parent_note": body.parent_note.clone(),
      "out_index": body.out_index.clone(),
      "blind": body.blind.clone(),
    };

    note
  }

  pub async fn store_note(&self, body: &NoteSchema) -> Result<NoteResponse, Error> {
    let document = self.create_note_document(body);

    let insert_result = match self.notes.insert_one(document, None).await {
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
  
    let note_doc = match self
      .notes
      .find_one(doc! {"_id": new_id}, None)
      .await
    {
      Ok(Some(doc)) => doc,
      Ok(None) => return Err(Error::from("User not found after insertion")),
      Err(e) => return Err(Error::from(e))
    };

    let note_owner = note_doc.get_str("owner")
    .map_err(|_| Error::from("owner pubkey not found"))?;

    let update_result = self.users.update_one(
      doc! { "pubkey": note_owner }, 
      doc! { "$push": { "notes": new_id } },
      None,
    ).await;

    if let Err(err) = update_result {
      eprintln!("Error updating user document: {:?}", err); 
      return Err(Error::from("Failed to update user document with note"));
    }

    Ok(self.doc_to_note(note_doc))
  }

  pub async fn get_user_notes(&self, user_pub_key: &str) -> Result<Vec<NoteSchema>, Error> {
    let filter = doc! { "owner": user_pub_key };

    let mut cursor: Cursor<Document> = self.notes
        .find(filter, None)
        .await
        .map_err(MyError::MongoError)?;
 
    let mut notes = Vec::new(); 

    while let Some(doc) = cursor.try_next().await? {
      let note = self.doc_to_note(doc);
      notes.push(note.note);
    }

    Ok(notes)
  }
}