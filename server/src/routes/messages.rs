use axum::{
  Extension,
  Json,
  http::StatusCode,
};
use crate::mongo::IOUServiceDB;
use crate::routes::schema::{MessageRequestSchema, UsernameRequest, MessageSchema};
use crate::routes::response::MessageSingleResponse;

#[axum::debug_handler]
pub async fn read_user_messages(
  Extension(db): Extension<IOUServiceDB>,
  Json(payload): Json<UsernameRequest>
) -> Result<Json<Vec<MessageSchema>>, StatusCode> {
  match db.get_unread_messages(&payload.username).await {
    Ok(messages) => Ok(Json(messages)),
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}

#[axum::debug_handler]
pub async fn send_message(Extension(db): Extension<IOUServiceDB>, Json(payload): Json<MessageRequestSchema>) -> Result<Json<MessageSingleResponse>, StatusCode> {
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
      Ok(Json(message_response))
    }
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}