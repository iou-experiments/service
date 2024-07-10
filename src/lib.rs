mod routes;

use axum::{
  routing::get,
  Router,
};

use routes::notes::{save_note, get_note};
use routes::messages::{send_message, read_message};
use routes::nullifier::verify_nullifier;

pub async fn run() {
  let app = Router::new()
  .route("/saveNote", get(save_note))
  .route("/getNote", get(get_note))
  .route("/verifyNullifier", get(verify_nullifier))
  .route("/sendMessage", get(send_message))
  .route("/readMessage", get(read_message));

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  axum::serve(listener, app).await.unwrap()
}
