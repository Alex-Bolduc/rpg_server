use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    EmptyName,
    Libsql(libsql::Error),
    De(de::value::Error),
    CharacterNotFound,
}

// To allow conversion (for await? for libsql)
impl From<libsql::Error> for Error {
    fn from(error: libsql::Error) -> Self {
        Error::Libsql(error)
    }
}

// To allow conversion (for await? for de::value::Error)
impl From<de::value::Error> for Error {
    fn from(error: de::value::Error) -> Self {
        Error::De(error)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("{}", self);
        let (status, body) = match self {
            Error::EmptyName => (StatusCode::BAD_REQUEST, "The name provided is empty."),
            Error::Libsql(_) | Error::De(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::INTERNAL_SERVER_ERROR
                    .canonical_reason()
                    .unwrap(),
            ),
            Error::CharacterNotFound => (StatusCode::NOT_FOUND, "This character does not exist."),
        };
        (status, body).into_response()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyName => {
                write!(f, "Empty name")
            }
            Error::Libsql(e) => {
                write!(f, "Libsql : {}", e)
            }
            Error::De(e) => {
                write!(f, "Deserialization : {}", e)
            }
            Error::CharacterNotFound => {
                write!(f, "User not found")
            }
        }
    }
}
