use ark_crypto_primitives::Error;
use bson::{doc, Document, Binary, Bson};
use mongodb::{Cursor, options::{ ClientOptions, FindOptions, ServerApi, ServerApiVersion, IndexOptions }, Client, Collection, IndexModel};
use std::{sync::{Arc, RwLock}, collections::HashMap, env};
use crate::routes::{
  error::{ConvertToDocError, CreateUserError, DatabaseError, InsertDocumentError, MyError},
  response::{
    MessageSingleResponse,
    NoteHistoryResponse,
    NoteResponse,
    NullifierResponse,
    NullifierResponseData,
    UserSingleResponse
  },
  schema::{
    ChallengeSchema, CreateUserSchema, MessageRequestSchema, MessageSchema, NoteHistorySaved, NoteNullifierSchema, NoteSchema, SaveNoteHistoryRequestSchema, SaveNoteRequestSchema, User
  }
};
use chrono::Utc;
use futures::stream::TryStreamExt;
use hex;
use rand::{Rng, distributions::Alphanumeric};
use ed25519_dalek::{PublicKey, Signature};
use error_stack::{Report, Result};

#[derive(Debug, Clone)]
pub struct IOUServiceDB {
  pub users_collection: Collection<User>,
  pub users: Collection<Document>,
  pub notes: Collection<Document>,
  pub notes_collection: Collection<NoteSchema>,
  pub note_history: Collection<Document>,
  pub note_history_collection: Collection<NoteHistorySaved>,
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
  ) -> Result<T, InsertDocumentError> {
    match collection.insert_one(document, None).await {
      Ok(insert_result) => {
        match insert_result.inserted_id.as_object_id() {
          Some(new_id) => {
            match collection.find_one(doc! {"_id": new_id}, None).await {
              Ok(Some(fetched_doc)) => Ok(transform(fetched_doc)),
              Ok(None) => Err(Report::new(InsertDocumentError)
                .attach_printable("Document not found after insertion")),
              Err(e) => Err(Report::new(InsertDocumentError)
                .attach_printable(format!("Failed to fetch inserted document: {}", e))),
            }
          },
          None => Err(Report::new(InsertDocumentError)
            .attach_printable("Failed to get inserted _id")),
        }
      },
      Err(e) => Err(Report::new(InsertDocumentError)
        .attach_printable(format!("Failed to insert document: {}", e))),
    }
  }

  async fn create_unique_index(&self, collection: &Collection<Document>, field: &str) -> Result<String, Error> {
    let options = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder()
      .keys(doc! {field: 1})
      .options(options)
      .build();

    let _ = collection.create_index(index, None).await;
    Ok("Success".to_owned())
  }

  fn get_current_timestamp(&self) -> i64 {
    Utc::now().timestamp()
  }

  // User
  fn create_user_document(&self, body: &CreateUserSchema) -> Result<Document, ConvertToDocError> {
    let user = doc! {
      "username": body.username.clone(),
      "pubkey": body.pubkey.clone(),
      "nonce": body.nonce.clone(),
      "messages": body.messages.clone(),
      "notes": body.notes.clone(),
      "has_double_spent": body.has_double_spent,
      "address": body.address.clone(),
    };
    
    Ok(user)
  }

  fn doc_to_user(&self, doc: Document) -> UserSingleResponse {
    let user = User {
      id: doc.get_str("_id").ok().map(|s| s.to_owned()),
      has_double_spent: doc.get_bool("has_double_spent").ok(),
      nonce: doc.get_str("nonce").ok().map(|s| s.to_owned()),
      username: doc.get_str("username").ok().map(|s| s.to_owned()),
      address: doc.get_str("address").ok().map(|s| s.to_owned()),
      pubkey: doc.get_str("pubkey").ok().map(|s| s.to_owned()),
      messages: doc.get_array("messages").ok().map(|arr| 
        arr.iter().filter_map(|bson| bson.as_str().map(|s| s.to_owned())).collect()),
      notes: doc.get_array("notes").ok().and_then(|arr| {
          let object_ids: Option<Vec<bson::oid::ObjectId>> = arr.iter()
              .map(|bson| match bson {
                  Bson::ObjectId(oid) => Some(*oid),
                  _ => None,
              })
              .collect();
          object_ids
      }),
    };

    UserSingleResponse {
      status: "success",
      user
    }
  }

  pub async fn get_user_with_username(&self, username: &str) -> Result<UserSingleResponse, DatabaseError> {
    // Find the user document
    let user_doc = match self.users.find_one(doc! {"username": username}, None).await {
      Ok(Some(doc)) => doc,
      Ok(None) => return Err(Report::new(DatabaseError::NotFoundError)
        .attach_printable(format!("User '{}' not found", username))),
      Err(e) => return Err(Report::new(DatabaseError::FetchError)
        .attach_printable(format!("Failed to fetch user '{}': {}", username, e))),
    };

    let user_res = self.doc_to_user(user_doc);

    Ok(UserSingleResponse {
      status: "success",
      user: user_res.user
    })
  }

  pub async fn get_user_with_address(&self, address: &str) -> Result<UserSingleResponse, DatabaseError> {
    // Find the user document
    let user_doc = match self.users.find_one(doc! {"address": address}, None).await {
      Ok(Some(doc)) => doc,
      Ok(None) => return Err(Report::new(DatabaseError::NotFoundError)
        .attach_printable(format!("User '{}' not found", address))),
      Err(e) => return Err(Report::new(DatabaseError::FetchError)
        .attach_printable(format!("Failed to fetch user '{}': {}", address, e))),
    };

    let user_res = self.doc_to_user(user_doc);

    Ok(UserSingleResponse {
      status: "success",
      user: user_res.user
    })
  }

  pub async fn create_user(&self, body: &CreateUserSchema) -> Result<UserSingleResponse, CreateUserError> {
    let document = match self.create_user_document(body) {
      Ok(doc) => doc,
      Err(e) => return Err(Report::new(CreateUserError)
        .attach_printable(format!("Failed to create user document: {}", e))),
    };

    match self.create_unique_index(&self.users, "username").await {
      Ok(_) => {},
      Err(e) => return Err(Report::new(CreateUserError)
        .attach_printable(format!("Failed to create unique index: {}", e))),
    }

    let user = match self.insert_and_fetch(&self.users, document, |doc| self.doc_to_user(doc).user).await {
      Ok(user) => user,
      Err(e) => return Err(Report::new(CreateUserError)
        .attach_printable(format!("Failed to insert and fetch user: {}", e))),
    };

    Ok(UserSingleResponse {
      status: "success",
      user,
    })
  }

  // Messages
  fn doc_to_message(&self, doc: Document) -> MessageSingleResponse {
    let message = MessageSchema {
      sender: doc.get_str("sender").ok().map(|s| s.to_owned()).unwrap(),
      recipient: doc.get_str("recipient").ok().map(|s| s.to_owned()).unwrap() ,
      message: doc.get_str("message").ok().map(|s| s.to_owned()).unwrap(),
      timestamp: doc.get_i64("timestamp").ok().unwrap(),
      attachment_id: doc.get("attachment_id").to_owned().cloned(),
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
  
  pub async fn send_message(&self, body: &MessageRequestSchema) -> Result<MessageSingleResponse, DatabaseError> {
    let document = self.create_message_document(body);

    let message = match self.insert_and_fetch(&self.messages, document, |doc| self.doc_to_message(doc).message).await {
      Ok(msg) => msg,
      Err(e) => return Err(Report::new(DatabaseError::InsertError)
        .attach_printable(format!("Failed to insert and fetch message: {}", e))),
    };

    match self.users.update_one(
      doc! { "username": message.recipient.clone() },
      doc! { "$push": { "messages": message._id.clone() } },
      None,
    ).await {
      Ok(_) => {},
      Err(e) => return Err(Report::new(DatabaseError::UpdateError)
        .attach_printable(format!("Failed to update user's messages: {}", e))),
    }

    Ok(MessageSingleResponse {
      status: "success",
      message
    })
  }

  pub async fn get_unread_messages(&self, username: &str) -> Result<Vec<MessageSchema>, DatabaseError> {
    let filter = doc! {
      "recipient": username,
      "read": false
    };
    let sort = doc! { "timestamp": 1 };
    let find_options = FindOptions::builder().sort(sort).build();

    let cursor = match self.messages.find(filter, Some(find_options)).await {
      Ok(cur) => cur,
      Err(e) => return Err(Report::new(DatabaseError::FetchError)
        .attach_printable(format!("Failed to fetch unread messages: {}", e))),
    };

    let messages: Vec<MessageSchema> = match cursor
      .try_filter_map(|doc| async move {
        let msg = self.doc_to_message(doc);
        match self.messages
          .update_one(
            doc! { "_id": msg.message._id.clone() },
            doc! { "$set": { "read": true } },
            None,
          )
          .await
        {
          Ok(_) => Ok(Some(msg.message)),
          Err(err) => {
            eprintln!("Error marking message as read: {:?}", err);
            Ok(None)
          }
        }
      })
      .try_collect()
      .await
    {
      Ok(msgs) => msgs,
      Err(e) => return Err(Report::new(DatabaseError::UpdateError)
        .attach_printable(format!("Failed to update message read status: {}", e))),
    };

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

  pub async fn store_nullifier(&self, body: &NoteNullifierSchema) -> Result<NullifierResponseData, DatabaseError> {
    let document = self.create_note_nullifier_document(body);

    match self.create_unique_index(&self.nullifiers, "state").await {
      Ok(_) => {},
      Err(e) => return Err(Report::new(DatabaseError::IndexCreationError)
        .attach_printable(format!("Failed to create unique index: {}", e))),
    }

    let nullifier = match self.insert_and_fetch(&self.nullifiers, document, |doc| self.doc_to_nullifier(doc).nullifier).await {
      Ok(null) => null,
      Err(e) => return Err(Report::new(DatabaseError::InsertError)
        .attach_printable(format!("Failed to insert and fetch nullifier: {}", e))),
    };

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
            println!("Nullifier and state combination is unique");
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

  pub async fn store_note(&self, body: &SaveNoteRequestSchema) -> Result<NoteResponse, DatabaseError> {
    let document = self.create_note_document(body);

    let note = match self.insert_and_fetch(&self.notes, document, |doc| self.doc_to_note(doc).note).await {
      Ok(n) => n,
      Err(e) => return Err(Report::new(DatabaseError::InsertError)
        .attach_printable(format!("Failed to insert and fetch note: {}", e))),
    };

    match self.users.update_one(
      doc! { "pubkey": note.owner.clone() },
      doc! { "$push": { "notes": note._id.clone() } },
      None,
    ).await {
      Ok(_) => {},
      Err(e) => return Err(Report::new(DatabaseError::UpdateError)
        .attach_printable(format!("Failed to update user's notes: {}", e))),
    }

    Ok(NoteResponse { status: "success", note })
  }

  pub async fn get_user_notes(&self, user_pub_key: &str) -> Result<Vec<NoteSchema>, DatabaseError> {
    let filter = doc! { "owner": user_pub_key };
    
    let mut cursor: Cursor<Document> = match self.notes.find(filter, None).await {
      Ok(cur) => cur,
      Err(e) => return Err(Report::new(DatabaseError::FetchError)
        .attach_printable(format!("Failed to fetch user notes: {}", e))),
    };

    let mut notes = Vec::new();
    
    loop {
      match cursor.try_next().await {
        Ok(Some(doc)) => {
          let note = self.doc_to_note(doc);
          notes.push(note.note);
        },
        Ok(None) => break,
        Err(e) => return Err(Report::new(DatabaseError::FetchError)
          .attach_printable(format!("Failed to fetch next note: {}", e))),
      }
    }

    Ok(notes)
  }

  // Notes History
  fn doc_to_note_history(&self, doc: Document) -> NoteHistoryResponse {
    let note_history = NoteHistorySaved {
      data: doc.get_array("data")
        .unwrap_or(&Vec::new())
        .iter()
        .flat_map(|bson| {
            match bson {
                bson::Bson::Binary(binary) => binary.bytes.to_vec(),
                _ => Vec::new(),
            }
        })
        .collect::<Vec<u8>>(),
      address: doc.get_str("address").ok().map(|s| s.to_owned()).unwrap_or_else(String::new),
      _id: doc.get("_id").to_owned().cloned(),
      sender: doc.get_str("sender").ok().map(|s| s.to_owned()).unwrap_or_else(String::new),
    };

    NoteHistoryResponse {
      status: "success", 
      note_history
    }
  }

  fn create_note_history_document(&self, body: SaveNoteHistoryRequestSchema) -> Document {
      let note_history = doc! {
          "data": Bson::Binary(Binary {
              subtype: bson::spec::BinarySubtype::Generic,
              bytes: body.data.clone(),
          }),
          "address": body.address.clone(),
          "sender": body.sender.clone()
      };
      note_history
  }

  pub async fn store_note_history(&self, body: SaveNoteHistoryRequestSchema) -> Result<NoteHistoryResponse, DatabaseError> {
    let document = self.create_note_history_document(body);

    let note_history = match self.insert_and_fetch(&self.note_history, document, |doc| self.doc_to_note_history(doc).note_history).await {
      Ok(history) => history,
      Err(e) => return Err(Report::new(DatabaseError::InsertError)
        .attach_printable(format!("Failed to insert and fetch note history: {}", e))),
    };

    match self.users.update_one(
      doc! { "address": note_history.address.clone() },
      doc! { "$push": { "notes": note_history._id.clone() } },
      None,
    ).await {
      Ok(_) => {},
      Err(e) => return Err(Report::new(DatabaseError::UpdateError)
        .attach_printable(format!("Failed to update user's notes: {}", e))),
    }

    Ok(NoteHistoryResponse {
      status: "success",
      note_history: note_history
    })
  }

  pub async fn get_note_history_for_user(&self, username: String) -> Result<Vec<NoteHistorySaved>, DatabaseError> {
    let user = self.get_user_with_username(&username).await
    .expect("user");

    let note_ids = user.user.notes.unwrap_or_default();

    // Update the user in the database to remove the notes
    self.users.update_one(
      doc! { "username": &username },
      doc! { "$set": { "notes": [] } },
      None
    ).await.expect("couldn't remove notes from user");

    let mut notes = Vec::new();
    for note_id in note_ids {
      match self.note_history.find_one(doc! { "_id": note_id }, None).await {
        Ok(Some(doc)) => {
            let note_history = NoteHistorySaved {
                data: doc.get_binary_generic("data")
                    .map(|b| b.clone())
                    .unwrap_or_default(),
                address: doc.get_str("address")
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                _id: doc.get("_id").cloned(),
                sender: doc.get_str("sender")
                  .map(|s| s.to_string())
                  .unwrap_or_default(),
            };
            notes.push(note_history);
        },
        Ok(None) => eprintln!("Note {} not found", note_id),
        Err(e) => eprintln!("Error finding note {}: {}", note_id, e),
    }
    }
    println!("{:#?}", notes);
    Ok(notes)
  }

  pub async fn create_and_transfer_note_history(
    &self,
    owner_username: String,
    recipient_username: &str,
    body: SaveNoteHistoryRequestSchema,
    message: String,
  ) -> Result<MessageSingleResponse, DatabaseError> {
    let to_save = SaveNoteHistoryRequestSchema {
      data: body.data.clone(),
      address: body.address.clone(),
      sender: owner_username.clone(),
    };
    let stored_note = self.store_note_history(to_save).await;
    let note_id = stored_note.expect("no note id").note_history._id.clone();
    
    let message = MessageRequestSchema {
      recipient: recipient_username.to_owned(),
      sender: owner_username.clone(),
      message: message.to_owned(),
      attachment_id: note_id.clone(),
    };

    self.users.update_one(
      doc! { "username": owner_username.to_owned() },
      doc! { "$pull": { "notes": note_id.clone() } },
      None,
    ).await.map_err(|e| Report::new(DatabaseError::UpdateError)
      .attach_printable(format!("Failed to remove note from current owner: {}", e)))?;
    match self.users.update_one(
      doc! { "username": recipient_username },
      doc! { "$push": { "notes": note_id.clone() } },
      None,
    ).await {
      Ok(_) => {},
      Err(e) => return Err(Report::new(DatabaseError::UpdateError)
        .attach_printable(format!("Failed to update user's notes: {}", e))),
    }

    let sent = self.send_message(&message);

    Ok(sent.await.expect("msg sent"))
  }

  //auth & challenges:  NOT IN USE DURING MVP
  pub async fn authenticate_user(
    &self,
    username: &str,
    signature_hex: &str,
    challenge_id: &str,
  ) -> Result<bool, DatabaseError> {
    let challenge = match self.get_challenge(Some(challenge_id), username).await {
      Ok(c) => c,
      Err(e) => return Err(Report::new(DatabaseError::FetchError)
        .attach_printable(format!("Failed to get challenge: {}", e))),
    };

    let user_doc = match self.users.find_one(doc! {"username": username}, None).await {
      Ok(Some(doc)) => doc,
      Ok(None) => return Err(Report::new(DatabaseError::AuthenticationError)
          .attach_printable("User not found")),
      Err(e) => return Err(Report::new(DatabaseError::FetchError)
        .attach_printable(format!("Failed to fetch user: {}", e))),
    };

    let public_key_str = match user_doc.get_str("pubkey") {
      Ok(key) => key,
      Err(e) => return Err(Report::new(DatabaseError::ConversionError)
        .attach_printable(format!("Failed to get pubkey: {}", e))),
    };

    let public_key_bytes = match hex::decode(public_key_str) {
      Ok(bytes) => bytes,
      Err(e) => return Err(Report::new(DatabaseError::ConversionError)
        .attach_printable(format!("Failed to decode pubkey: {}", e))),
    };

    let public_key = match PublicKey::from_bytes(&public_key_bytes) {
      Ok(key) => key,
      Err(e) => return Err(Report::new(DatabaseError::ConversionError)
        .attach_printable(format!("Failed to create PublicKey: {}", e))),
    };

    let signature_bytes = match hex::decode(signature_hex) {
      Ok(bytes) => bytes,
      Err(e) => return Err(Report::new(DatabaseError::ConversionError)
        .attach_printable(format!("Failed to decode signature: {}", e))),
    };

    let signature = match Signature::from_bytes(&signature_bytes) {
      Ok(sig) => sig,
      Err(e) => return Err(Report::new(DatabaseError::ConversionError)
        .attach_printable(format!("Failed to create Signature: {}", e))),
    };

    let is_valid = public_key.verify_strict(&challenge, &signature).is_ok();
    Ok(is_valid)
  }

  pub fn insert_session(&self, session_id: String, username: String) {
    self.sessions.write().unwrap().insert(session_id, username);
  }

  pub async fn get_challenge(
    &self,
    challenge_id: Option<&str>,
    username: &str,
  ) -> Result<Vec<u8>, DatabaseError> {
    if let Some(challenge_id) = challenge_id {
      match self.challenges.find_one(
        doc! {"challenge_id": challenge_id, "expires_at": { "$gt": Utc::now() }},
        None
      ).await {
        Ok(Some(doc)) => {
          let challenge = self.document_to_challenge(doc);
          return Ok(challenge.challenge_id.as_bytes().to_vec())
        },
        Ok(None) => {},
        Err(e) => return Err(Report::new(DatabaseError::FetchError)
         .attach_printable(format!("Failed to fetch challenge: {}", e))),
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

    match self.challenges_collection.insert_one(new_challenge, None).await {
      Ok(_) => Ok(challenge_id.as_bytes().to_vec()),
      Err(e) => Err(Report::new(DatabaseError::InsertError)
        .attach_printable(format!("Failed to insert new challenge: {}", e))),
    }
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