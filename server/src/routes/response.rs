use crate::routes::schema::User;
use serde::Serialize;
use crate::routes::schema::MessageSchema;

#[derive(Debug, Serialize)]
pub struct UserSingleResponse {
    pub status: &'static str,
    pub user: User,
}

#[derive(Debug, Serialize)]
pub struct MessageSingleResponse {
    pub status: &'static str,
    pub message: MessageSchema,
}