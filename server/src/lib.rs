mod routes;
mod crypto;
mod mongo;

use axum::Extension;
use axum::{
  routing::post,
  routing::get,
  Router,
};

use mongo::IOUServiceDB;
use routes::notes::{save_note, get_note};
use routes::messages::{send_message, read_message};
use routes::nullifier::verify_nullifier;
use routes::users::get_user_with_username;

pub async fn run() {
  let mongo = IOUServiceDB::init().await;
  let app = Router::new()

  .route("/saveNote", post(save_note))
  .route("/getNote", get(get_note))
  .route("/verifyNullifier", get(verify_nullifier))
  .route("/sendMessage", post(send_message))
  .route("/readMessage", get(read_message))
  .route("/getUser", get(get_user_with_username))
  .layer(Extension(mongo));

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  axum::serve(listener, app).await.unwrap()
}
