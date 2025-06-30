use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    EmptyName,
    Libsql(libsql::Error),
    De(de::value::Error),
    CharacterNotFound,
    ItemNotFound,
    ItemInstanceNotFound,
    AuctionNotFound,
    AuctionNotActive,
    InsufficientGold,
    IncorrectBuyer,
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
            Error::ItemNotFound => (StatusCode::NOT_FOUND, "This item does not exist."),
            Error::ItemInstanceNotFound => {
                (StatusCode::NOT_FOUND, "This item instance does not exist.")
            }
            Error::AuctionNotFound => (StatusCode::NOT_FOUND, "This auction does not exist."),
            Error::AuctionNotActive => (StatusCode::NOT_FOUND, "This auction is not active."),
            Error::InsufficientGold => (
                StatusCode::FORBIDDEN,
                "The buyer does not have enough gold.",
            ),
            Error::IncorrectBuyer => (
                StatusCode::FORBIDDEN,
                "The buyer cannot be the auction's owner.",
            ),
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
                write!(f, "Character not found")
            }
            Error::ItemNotFound => {
                write!(f, "Item not found")
            }
            Error::ItemInstanceNotFound => {
                write!(f, "Item instance not found")
            }
            Error::AuctionNotFound => {
                write!(f, "Auction not found")
            }
            Error::AuctionNotActive => {
                write!(f, "Auction not active")
            }
            Error::InsufficientGold => {
                write!(f, "Not enough gold")
            }
            Error::IncorrectBuyer => {
                write!(f, "Incorrect buyer")
            }
        }
    }
}
