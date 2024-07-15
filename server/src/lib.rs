pub mod routes;
mod crypto;
pub mod mongo;

use axum::Extension;
use axum::{
  routing::post,
  routing::get,
  Router,
};

use mongo::IOUServiceDB;
use routes::notes::{save_note, get_note};
use routes::messages::{send_message, read_user_messages};
use routes::nullifier::verify_nullifier;
use routes::users::{get_user_with_username, create_user};

pub async fn run() {
  let mongo = IOUServiceDB::init().await;
  let app = Router::new()
  .route("/createUser", post(create_user))
  .route("/saveNote", post(save_note))
  .route("/getNote", get(get_note))
  .route("/verifyNullifier", get(verify_nullifier))
  .route("/sendMessage", post(send_message))
  .route("/readMessage", get(read_user_messages))
  .route("/getUser", get(get_user_with_username))
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
