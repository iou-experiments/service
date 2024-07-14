use crate::mongo::User;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct UserSingleResponse {
    pub status: &'static str,
    pub user: User,
}