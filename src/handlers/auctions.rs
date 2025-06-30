use crate::{
    AppState,
    errors::{Error, Result},
    handlers::characters::{Character, get_character_libsql_query},
    into_rows,
};
use axum::{
    extract::{self, Extension, Json, Path, Query, Request, State},
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
    pub id: Uuid,
    pub auctioned_item_id: Uuid,
    pub seller_name: String,
    pub creation_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub price: u64,
    pub status: AuctionStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AuctionStatus {
    Active,
    Sold,
    Expired,
}

impl AuctionStatus {
    pub fn to_string(&self) -> String {
        match self {
            AuctionStatus::Active => "active".to_string(),
            AuctionStatus::Sold => "sold".to_string(),
            AuctionStatus::Expired => "expired".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuctionStatusQuery {
    status: Option<AuctionStatus>,
}

// =========================Query functions=========================
async fn get_auctions_libsql_query(
    state: &State<AppState>,
    query_status: Query<AuctionStatusQuery>,
) -> Result<Vec<Auction>> {
    let query;
    if let Some(status) = query_status.status {
        query = state
            .conn
            .query(
                "SELECT * FROM auctions WHERE status = ?1",
                [status.to_string()],
            )
            .await?;
    } else {
        query = state.conn.query("SELECT * FROM auctions", ()).await?;
    }

    let auctions: Vec<Auction> = into_rows(query).await?;
    Ok(auctions)
}

pub async fn get_auction_libsql_query(
    state: &State<AppState>,
    id: &Uuid,
) -> Result<Option<Auction>> {
    let mut query = state
        .conn
        .query("SELECT * FROM auctions WHERE id = ?1", [id.to_string()])
        .await?;
    let auction = query.next().await?; //None if there are no more rows
    auction
        .map(|row| from_row(&row).map_err(Error::from))
        .transpose()
}

// =========================Handlers=========================
pub async fn get_auctions(
    state: State<AppState>,
    query_status: Query<AuctionStatusQuery>,
) -> Result<Json<Vec<Auction>>> {
    let auctions = get_auctions_libsql_query(&state, query_status).await?;
    Ok(Json(auctions))
    // let mut header = HeaderMap::new();
    // header.insert(
    //     CONTENT_TYPE,
    //     "application/json".parse::<HeaderValue>().unwrap(),
    // );
    // (header, serde_json::to_string(&characters).unwrap())
}

pub async fn get_auction(Extension(auction): Extension<Auction>) -> Json<Auction> {
    Json(auction)
}

pub async fn post_auction(
    state: State<AppState>,
    Extension(mut auction): Extension<Auction>,
    Json(buyer): Json<Character>,
) -> Result<(StatusCode, Option<Json<Auction>>)> {
    let Some(buyer) = get_character_libsql_query(&state, &buyer.name).await? else {
        return Err(Error::CharacterNotFound);
    };

    let present = Utc::now();
    if present >= auction.end_date {
        return Err(Error::AuctionNotActive);
    }

    if auction.price > buyer.gold {
        return Err(Error::InsufficientGold);
    }

    if buyer.name == auction.seller_name {
        return Err(Error::IncorrectBuyer);
    }

    let Some(owner) = get_character_libsql_query(&state, &auction.seller_name).await? else {
        return Err(Error::CharacterNotFound);
    };

    state
        .conn
        .execute(
            "UPDATE characters SET gold = ?1 WHERE name = ?2",
            (buyer.gold - auction.price, buyer.name.as_str()),
        )
        .await?;
    state
        .conn
        .execute(
            "UPDATE characters SET gold = ?1 WHERE name = ?2",
            (owner.gold + auction.price, auction.seller_name.as_str()),
        )
        .await?;

    auction.status = AuctionStatus::Sold;

    Ok((StatusCode::CREATED, Some(Json(auction))))
}

// =========================Middleware=========================
pub async fn middleware_auction_exists(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Response {
    let response = get_auction_libsql_query(&state, &id).await;
    match response {
        Ok(None) => Error::AuctionNotFound.into_response(),
        Err(e) => e.into_response(),
        Ok(Some(auction)) => {
            request.extensions_mut().insert(auction);
            next.run(request).await
        }
    }
}
