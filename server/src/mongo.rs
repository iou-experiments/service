use mongodb::{ bson::doc, options::{ ClientOptions, ServerApi, ServerApiVersion }, Client, Collection };
use std::env;

pub struct SpentNote {
  nullifier: (),
  user_pub_key: (),
}

pub struct IOUServiceDB {
  users: Collection<()>,
  note_history: Collection<()>,
  messages: Collection<()>,
  nullifiers: Collection<()>
}

impl IOUServiceDB {
  pub async fn init() -> Self {
    let uri = env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
    println!("{}", uri);
    let mut client_options = ClientOptions::parse(uri).await.unwrap();
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("admin");
    println!("Pinged your deployment. You successfully connected to MongoDB!");
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
}