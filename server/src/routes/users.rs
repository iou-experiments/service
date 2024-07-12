use axum::extract::State;
use mongo::IOUServiceDB;

pub async fn get_user_with_username(
  username: String,
  db: &State<IOUServiceDB>
) {
  match db.get_user(&username).await {
    Some(user) => Ok(Json(user)),
    None => Err("no user")
  }
}