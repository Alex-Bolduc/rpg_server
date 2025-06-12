use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    EmptyName,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::EmptyName => {
                "The name provided is empty.";
                (StatusCode::BAD_REQUEST).into_response()
            }
        }
    }
}
