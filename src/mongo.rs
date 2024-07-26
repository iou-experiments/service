use ark_crypto_primitives::Error;
use bson::{doc, Document};
use mongodb::{ Cursor, options::{ ClientOptions, FindOptions, ServerApi, ServerApiVersion }, Client, Collection};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use crate::routes::{
  error::MyError,
  response::{
    MessageSingleResponse,
    NoteHistoryResponse,
    NoteResponse,
    NullifierResponse,
    NullifierResponseData,
    UserSingleResponse
  },
  schema::{
    ChallengeSchema, CreateUserSchema, MessageRequestSchema, NoteHistory, SaveNoteRequestSchema, User
  }
};

use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use std::env;
use crate::routes::schema::{NoteSchema, MessageSchema, NoteNullifierSchema};
use chrono::Utc;
use futures::stream::TryStreamExt;
use hex;
use rand::Rng;
use ed25519_dalek::{PublicKey, Signature};
use rand::distributions::Alphanumeric;

#[derive(Debug, Clone)]
pub struct IOUServiceDB {
  pub users_collection: Collection<User>,
  pub users: Collection<Document>,
  pub notes: Collection<Document>,
  pub notes_collection: Collection<NoteSchema>,
  pub note_history: Collection<Document>,
  pub note_history_collection: Collection<NoteHistory>,
  pub messages: Collection<Document>,
  pub messages_collection: Collection<MessageSchema>,
  pub nullifiers: Collection<Document>,
  pub nullifiers_collection: Collection<NoteNullifierSchema>,
  pub challenges_collection: Collection<ChallengeSchema>,
  pub challenges: Collection<Document>,
  pub sessions: Arc<RwLock<HashMap<String, String>>>, 
}

impl IOUServiceDB {
  pub async fn init() -> Self {
    let uri = env::var("MONGODB_URI").map_err(|_| MyError::InternalServerError("MONGODB_URI not set".to_string()));
    let mut client_options = ClientOptions::parse(uri.expect("uri is set")).await.unwrap();
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
    // note history
    let note_history = db.collection::<Document>("note_history");
    let note_history_collection = db.collection("note_history");
    //messages
    let messages = db.collection::<Document>("messages");
    let messages_collection = db.collection("messages");
    // betrayal detection system
    let nullifiers = db.collection::<Document>("nullifiers");
    let nullifiers_collection = db.collection("nullifiers");
    // auth challenge
    let challenges_collection = db.collection("challenges");
    let challenges = db.collection::<Document>("challenges");
    let sessions = Arc::new(RwLock::new(HashMap::new()));

    Self {
      users,
      users_collection,
      notes,
      notes_collection,
      messages,
      messages_collection,
      nullifiers,
      nullifiers_collection,
      note_history,
      note_history_collection,
      challenges,
      challenges_collection,
      sessions
    }
  }


  // Helpers

  async fn insert_and_fetch<T>(
    &self,
    collection: &Collection<Document>,
    document: Document,
    transform: impl Fn(Document) -> T,
  ) -> Result<T, Error> {
    let insert_result = collection.insert_one(document, None).await?;
    
    let new_id = insert_result
      .inserted_id
      .as_object_id()
      .ok_or_else(|| Error::from("Failed to get inserted _id"))?;

    let fetched_doc = collection
      .find_one(doc! {"_id": new_id}, None)
      .await?
      .ok_or_else(|| Error::from("Document not found after insertion"))?;

    Ok(transform(fetched_doc))
  }

  async fn create_unique_index(&self, collection: &Collection<Document>, field: &str) -> Result<String, Error> {
    let options = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder()
      .keys(doc! {field: 1})
      .options(options)
      .build();

    collection.create_index(index, None).await?;
    Ok("Success".to_owned())
  }

  fn get_current_timestamp(&self) -> i64 {
    Utc::now().timestamp()
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
    let _ = self.create_unique_index(&self.users, "username").await;
    let user = self.insert_and_fetch(&self.users, document, |doc| self.doc_to_user(doc).user).await?;

    Ok(UserSingleResponse {
      status: "success",
      user
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
  
  pub async fn send_message(&self, body: &MessageRequestSchema) -> Result<MessageSingleResponse, Error> {
    let document = self.create_message_document(body);

    let message = self.insert_and_fetch(&self.messages, document, |doc| self.doc_to_message(doc).message).await?;

    self.users.update_one(
      doc! { "username": message.recipient.clone() }, 
      doc! { "$push": { "messages": message._id.clone() } },
      None,
    ).await?;

    Ok(MessageSingleResponse {
      status: "success",
      message
    })
  }

  pub async fn get_unread_messages(&self, username: &str) -> Result<Vec<MessageSchema>, Error> {
    let filter = doc! {
      "recipient": username,
      "read": false
    };

    let sort = doc! { "timestamp": 1 };
    let find_options = FindOptions::builder().sort(sort).build();

    let messages: Vec<MessageSchema> = self.messages
      .find(filter, Some(find_options))
      .await?
      .try_filter_map(|doc| async move {
        let msg = self.doc_to_message(doc);
        let update_result = self.messages
            .update_one(
              doc! { "_id": msg.message._id.clone() },
              doc! { "$set": { "read": true } },
              None,
            )
            .await;

        match update_result {
          Ok(_) => Ok(Some(msg.message)),
          Err(err) => {
            eprintln!("Error marking message as read: {:?}", err);
            Ok(None)
          }
        }
      })
      .try_collect()
      .await?;

    Ok(messages)
  }

  // Nullifiers 
  fn doc_to_nullifier(&self, doc: Document) -> NullifierResponseData {
    let nullifier = NoteNullifierSchema {
      nullifier: doc.get_str("nullifier").ok().map(|s| s.to_owned()).unwrap(),
      note: doc.get_str("note").ok().map(|s| s.to_owned()).unwrap(),
      step: doc.get_i32("step").ok().unwrap(),
      owner: doc.get_str("owner").ok().map(|s| s.to_owned()).unwrap(),
      state: doc.get_str("state").ok().map(|s| s.to_owned()).unwrap(),
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
      "state": body.state.clone()
    };

    nullifier
  }

  pub async fn store_nullifier(&self, body: &NoteNullifierSchema) -> Result<NullifierResponseData, Error> {
    let document = self.create_note_nullifier_document(body);

    let _ = self.create_unique_index(&self.nullifiers, "state").await;
    let nullifier = self.insert_and_fetch(&self.nullifiers, document, |doc| self.doc_to_nullifier(doc).nullifier).await?;

    Ok(NullifierResponseData {
      status: "success",
      nullifier
    })
  }

  pub async fn get_nullifier(&self, nullifier: &str, expected_state: &str) -> NullifierResponse { 
    let nullifier_doc = self
        .nullifiers
        .find_one(doc! {"nullifier": nullifier}, None)
        .await;

    match nullifier_doc {
      Ok(Some(doc)) => {
        if let Some(state) = doc.get_str("state").ok() { 
          if state == expected_state {
            if let Ok(owner) = doc.get_str("owner") {
              let update_result = self.users
                .update_one(
                    doc! {"username": owner},
                    doc! {"$set": {"has_double_spent": true}},
                    None,
                )
                .await;

              if let Err(err) = update_result {
                eprintln!("Error updating user: {:?}", err);
              }
            } else {
              eprintln!("Owner not found in nullifier document.");
            }

            println!("WARNING: USER IS ATTEMPTING TO DOUBLE SPEND, we have flagged their account.");
            return NullifierResponse::Ok(self.doc_to_nullifier(doc).nullifier);
          } else {
            return NullifierResponse::Error; 
          }
        } else {
            eprintln!("State field not found in nullifier document.");
            return NullifierResponse::Error; 
        }
      }
      Ok(None) => NullifierResponse::NotFound, 
      Err(err) => {
          eprintln!("Error getting nullifier: {:?}", err);
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
      _id: doc.get("_id").to_owned().cloned()
    };

    NoteResponse { status: "success", note }
  }

  fn create_note_document(&self, body: &SaveNoteRequestSchema) -> Document {
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

  pub async fn store_note(&self, body: &SaveNoteRequestSchema) -> Result<NoteResponse, Error> {
    let document = self.create_note_document(body);

    let note = self.insert_and_fetch(&self.notes, document, |doc| self.doc_to_note(doc).note).await?;

    self.users.update_one(
      doc! { "pubkey": note.owner.clone() }, 
      doc! { "$push": { "notes": note._id.clone() } },
      None,
    ).await?;

    Ok(NoteResponse { status: "success", note })
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

  // Notes History
  fn doc_to_note_history(&self, doc: Document) -> NoteHistoryResponse {
    let current_note = doc.get_document("current_note")
      .map(|note_doc| self.doc_to_note(note_doc.clone()).note)
      .expect("Failed to get current_note");

    let note_history = NoteHistory {
      current_note: SaveNoteRequestSchema{
        asset_hash: current_note.asset_hash,
        owner: current_note.owner,
        value: current_note.value,
        step: current_note.step,
        parent_note: current_note.parent_note,
        out_index: current_note.out_index,
        blind: current_note.blind,
      },
      asset: doc.get_str("asset").ok().map(|s| s.to_owned()).unwrap(),
      steps: doc.get_array("steps").ok().map(|arr| {
          arr.iter().filter_map(|bson| bson.as_str().map(|s| s.to_owned())).collect()
      }).unwrap_or_else(Vec::new),
      sibling: doc.get_str("sibling").ok().map(|s| s.to_owned()).unwrap_or_else(String::new),
    };

    NoteHistoryResponse {
      status: "success", 
      note_history
    }
  }

  fn create_note_history_document(&self, body: NoteHistory ) -> Document {
    let note_history = doc! {
      "asset": body.asset.clone(),
      "steps": body.steps,
      "current_note": self.create_note_document(&body.current_note),
      "sibling": body.sibling.clone()
    };

    note_history
  }

  pub async fn store_note_history(&self, body: NoteHistory) -> Result<NoteHistoryResponse, Error> {
    let document = self.create_note_history_document(body);

    let note_history = self.insert_and_fetch(&self.note_history, document, |doc| self.doc_to_note_history(doc).note_history).await?;

    Ok(NoteHistoryResponse {
      status: "success",
      note_history
    })
  }

  pub async fn create_and_transfer_note_history(
    &self,
    current_owner_username: &str,
    recipient_username: &str,
    body: NoteHistory,
    message: String,
  ) -> MessageSingleResponse {
    let stored_note = self.store_note(&body.current_note).await;
    let message = MessageRequestSchema {
      recipient: recipient_username.to_owned(),
      sender: current_owner_username.to_owned(),
      message: message.to_owned(),
      attachment_id: stored_note.unwrap().note._id,
    };

    let sent = self.send_message(&message);

    sent.await.expect("msg sent")
  }

  //auth & challenges
  pub async fn authenticate_user(
    &self,
    username: &str,
    signature_hex: &str, 
    challenge_id: &str,
  ) -> Result<bool, Error> {

    let challenge = self.get_challenge(Some(challenge_id), username).await; // Implement this
    let user_doc = self.users.find_one(doc! {"username": username}, None).await?;
    let public_key_bytes = hex::decode(
        user_doc.unwrap().get_str("pubkey")?
    ).map_err(|_| Error::from("owner pubkey not found"))?;

    let public_key: PublicKey = PublicKey::from_bytes(&public_key_bytes)?; 

    let signature_bytes = hex::decode(signature_hex).map_err(|_| Error::from("no existing challenge"))?;
    let signature: Signature = Signature::from_bytes(&signature_bytes)?;

    let is_valid = public_key.verify_strict(&challenge.unwrap(), &signature).is_ok();

    if is_valid {
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn insert_session(&self, session_id: String, username: String) {
    self.sessions.write().unwrap().insert(session_id, username);
  }

  pub async fn get_challenge(
    &self, 
    challenge_id: Option<&str>, // Make challenge_id optional
    username: &str,
) -> Result<Vec<u8>, Error> { 
    if let Some(challenge_id) = challenge_id {
      // 1. If challenge_id is provided, try to find it in the database
      let existing_challenge = self.challenges.find_one(
        doc! {"challenge_id": challenge_id, "expires_at": { "$gt": Utc::now() }},
        None
      ).await
      .map_err(|_| Error::from("database error"))?;

      if let Some(doc) = existing_challenge {
        let challenge = self.document_to_challenge(doc); // Assuming you have this function
        return Ok(challenge.challenge_id.as_bytes().to_vec());
      } 
    } 

    let challenge_id: String = rand::thread_rng()
      .sample_iter(&Alphanumeric)
      .take(32) 
      .map(char::from)
      .collect();

    let new_challenge = ChallengeSchema {
      challenge_id: challenge_id.clone(),
      user_id: username.to_owned(), 
      created_at: self.get_current_timestamp(),
      expires_at: self.get_current_timestamp() + 300, 
    };

    self.challenges_collection.insert_one(new_challenge, None)
      .await
      .map_err(|_| Error::from("database error"))?;

    Ok(challenge_id.as_bytes().to_vec())
  }

  fn document_to_challenge(&self, doc: Document) -> ChallengeSchema {
    let challenge = ChallengeSchema {
      challenge_id: doc.get_str("challenge_id").ok().map(|s| s.to_owned()).unwrap(),
      user_id: doc.get_str("user_id").ok().map(|s| s.to_owned()).unwrap(),
      created_at: doc.get_i64("created_at").ok().unwrap(),
      expires_at: doc.get_i64("expires_at").ok().unwrap(),
    };

    challenge
  }
}