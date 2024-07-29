use thiserror::Error;
use mongodb::bson;
use error_stack::Result;
use std::error::Error;

#[derive(Debug)]
pub struct ConvertToDocError;
impl std::fmt::Display for ConvertToDocError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Convert to document failed")
    }
}

impl Error for ConvertToDocError {}

#[derive(Debug)]
pub struct ConvertFromDocError;

impl std::fmt::Display for ConvertFromDocError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Convert from document failed")
    }
}

impl Error for ConvertFromDocError {}


#[derive(Debug)]
pub struct InsertDocumentError;

impl std::fmt::Display for InsertDocumentError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Inserting Document failed")
    }
}

impl std::error::Error for InsertDocumentError {}

#[derive(Debug)]
pub struct CreateUserError;

impl std::fmt::Display for CreateUserError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Create User failed, username not unique.")
    }
}

impl std::error::Error for CreateUserError {}

#[derive(Debug)]
pub enum DatabaseError {
    InsertError,
    FetchError,
    UpdateError,
    ConversionError,
    IndexCreationError,
    AuthenticationError,
    NotFoundError,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::InsertError => write!(f, "Failed to insert data"),
            DatabaseError::FetchError => write!(f, "Failed to fetch data"),
            DatabaseError::UpdateError => write!(f, "Failed to update data"),
            DatabaseError::ConversionError => write!(f, "Failed to convert data"),
            DatabaseError::IndexCreationError => write!(f, "Failed to create index"),
            DatabaseError::AuthenticationError => write!(f, "Authentication failed"),
            DatabaseError::NotFoundError => write!(f, "Data not found"),
        }
    }
}

impl std::error::Error for DatabaseError {}

#[derive(Error, Debug)]
pub enum MyError {
    #[error("MongoDB error: {0}")]
    MongoError(#[from] mongodb::error::Error),

    #[error("MongoDB query error: {0}")]
    MongoQueryError(String),

    #[error("BSON serialization error: {0}")]
    BsonSerializationError(#[from] bson::ser::Error),

    #[error("BSON deserialization error: {0}")]
    BsonDeserializationError(#[from] bson::de::Error),

    #[error("Data validation error: {0}")]
    ValidationError(String),

    #[error("Invalid ID: {0}")]
    InvalidIdError(String),

    #[error("Not found: {0}")]
    NotFoundError(String),

    #[error("Unauthorized: {0}")]
    UnauthorizedError(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

pub type MyResult<T> = Result<T, MyError>;

#[derive(serde::Serialize)]
pub struct ErrorResponse {
   pub error: String,
}