pub mod routes;
pub mod mongo;

use axum::Extension;
use axum::{
  routing::post,
  routing::get,
  Router,
};

use mongo::IOUServiceDB;
use routes::notes::{create_and_transfer_note_history, get_notes, save_note};
use routes::messages::{send_message, read_user_messages};
use routes::nullifier::{store_nullifier, verify_nullifier};
use routes::users::{
  get_user_with_username,
  create_user,
  create_and_send_challenge, 
};

pub async fn run() {
  let mongo = IOUServiceDB::init().await;
  let app = Router::new()
  // user routes
  .route("/get_user", get(get_user_with_username))
  .route("/create_user", post(create_user))
  // verifier routes
  .route("/verify_nullifier", get(verify_nullifier))
  .route("/auth", post(create_and_send_challenge))
  // .route("/get_session", post(verify_challenge))
    // note routes
  .route("/store_note", post(save_note))
  .route("/get_notes", get(get_notes))
  // message routes
  .route("/send_message", post(send_message))
  .route("/read_messages", get(read_user_messages))
  // store
  .route("/store_nullifier", post(store_nullifier))
  // create and transfer notes history
  .route("/create_and_transfer_note_history", post(create_and_transfer_note_history))
  // fallback, state, and db
  .fallback(handler_404)
  .layer(Extension(mongo));

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  axum::serve(listener, app).await.unwrap()
}

#[axum::debug_handler]
async fn handler_404(method: axum::http::Method, uri: axum::http::Uri) -> impl axum::response::IntoResponse {
  println!("404 for {} {}", method, uri);
  (axum::http::StatusCode::NOT_FOUND, "Not Found")
}
