pub async fn get_note() -> String {
  "got note!".to_owned()
}

pub async fn save_note(note: String) -> Result<String, String> {
  println!("Note saved! Serialized data: {}", note); // Print a message with the serialized data
  Ok("serizlised".to_owned())
}