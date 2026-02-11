use std::io;
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] Utf8Error),

    #[error("Empty request")]
    EmptyRequest,

    #[error("Invalid request line")]
    InvalidRequestLine,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Template error: {0}")]
    Template(#[from] askama::Error),

    #[error("Slug validation error: {0}")]
    ValidationError(String),

    #[error("Multipart parsing error: {0}")]
    MultipartError(String),
}

impl Error {
    pub fn status_code(&self) -> u16 {
        match self {
            Error::BadRequest(_) => 400,
            Error::NotFound(_) => 404,
            Error::Io(_) => 500,
            Error::Utf8(_) => 400,
            Error::EmptyRequest => 400,
            Error::InvalidRequestLine => 400,
            Error::InternalServerError(_) => 500,
            Error::Template(_) => 500,
            Error::ValidationError(_) => 400,
            Error::MultipartError(_) => 400,
        }
    }
}
