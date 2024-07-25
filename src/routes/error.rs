use thiserror::Error;
use mongodb::bson;

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