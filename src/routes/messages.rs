use axum::{
  Extension,
  Json,
  http::StatusCode,
  response::IntoResponse
};
use crate::mongo::IOUServiceDB;
use crate::routes::schema::{MessageRequestSchema, UsernameRequest};
use super::error::ErrorResponse;

#[axum::debug_handler]
pub async fn read_user_messages(
  Extension(db): Extension<IOUServiceDB>,
  Json(payload): Json<UsernameRequest>
) -> impl IntoResponse {
  match db.get_unread_messages(&payload.username).await {
    Ok(messages) => {
      println!("{:#?}", messages);
      (StatusCode::OK, Json(messages)).into_response()
    },
    Err(err) => {
      let error_message = format!("Failed to get user messages: {}", err);
      (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: error_message })).into_response()
    }
  }
}

#[axum::debug_handler]
pub async fn send_message(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<MessageRequestSchema>) -> impl IntoResponse {
  let message = MessageRequestSchema {
    recipient: payload.recipient,
    sender: payload.sender,
    message: payload.message,
    attachment_id: payload.attachment_id
  };
  println!("{:#?}", message);
  match db.send_message(&message).await {
    Ok(message_response) => {
      println!("{:#?}", message_response);
      (StatusCode::OK, Json(message_response)).into_response()
    },
    Err(err) => {
      let error_message = format!("Failed to get send message: {}", err);
      (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: error_message })).into_response()
    }
  }
}