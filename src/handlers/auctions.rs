use crate::{
    AppState,
    errors::{Error, Result},
    into_rows,
};
use axum::{
    extract::{Extension, Json, Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, TimeZone, Utc};
use libsql::de::from_row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Auction {
    id: Uuid,
    creation_date: DateTime<Utc>,
    end_date: DateTime<Tc>,
    price: u64,
    status: AuctionStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum AuctionStatus {
    Active,
    Sold,
    Expired,
}
