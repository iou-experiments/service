use crate::mongo::User;

pub struct UserSingleResponse {
    pub status: &'static str,
    pub user: User,
}