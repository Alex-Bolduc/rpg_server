use crate::{
    AppState,
    errors::{Error, Result},
    handlers::auctions::{Auction, get_auction_libsql_query},
    into_rows,
};
use axum::{
    extract::{Extension, Json, Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use libsql::de::from_row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewItem {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemInstance {
    pub id: Uuid,
    pub item_name: String,
    pub item_id: Uuid,
    pub owner_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemNameUpdate {
    name: String,
}

// =========================Query functions=========================
async fn get_items_libsql_query(state: &State<AppState>) -> Result<Vec<Item>> {
    let query = state.conn.query("SELECT * FROM items", ()).await?;
    let items: Vec<Item> = into_rows(query).await?;
    Ok(items)
}

pub async fn get_item_libsql_query(state: &State<AppState>, id: &Uuid) -> Result<Option<Item>> {
    let mut query = state
        .conn
        .query("SELECT * FROM items WHERE id = ?1", [id.to_string()])
        .await?;
    let item = query.next().await?; //None if there are no more rows
    item.map(|row| from_row(&row).map_err(Error::from))
        .transpose()
}

pub async fn get_item_instance_libsql_query(
    state: &State<AppState>,
    id: &Uuid,
) -> Result<Option<ItemInstance>> {
    let mut query = state
        .conn
        .query(
            "SELECT * FROM items_instances WHERE id = ?1",
            [id.to_string()],
        )
        .await?;
    let item = query.next().await?; //None if there are no more rows
    item.map(|row| from_row(&row).map_err(Error::from))
        .transpose()
}

async fn get_item_auctions_libsql_query(
    state: &State<AppState>,
    id: &Uuid,
) -> Result<Vec<Auction>> {
    let query = state
        .conn
        .query(
            "SELECT * FROM auctions WHERE auctioned_item_id = ?1",
            [id.to_string()],
        )
        .await?;
    let auctions: Vec<Auction> = into_rows(query).await?;
    Ok(auctions)
}

// =========================Handlers=========================
pub async fn get_items(state: State<AppState>) -> Result<Json<Vec<Item>>> {
    let items = get_items_libsql_query(&state).await?;
    Ok(Json(items))
    // let mut header = HeaderMap::new();
    // header.insert(
    //     CONTENT_TYPE,
    //     "application/json".parse::<HeaderValue>().unwrap(),
    // );
    // (header, serde_json::to_string(&characters).unwrap())
}

pub async fn post_item(
    state: State<AppState>,
    Json(new_item): Json<NewItem>,
) -> Result<(StatusCode, Json<Item>)> {
    if new_item.name.is_empty() {
        return Err(Error::EmptyName);
    }

    let new_id = Uuid::new_v4();
    let item = Item {
        id: new_id,
        name: new_item.name,
    };

    state
        .conn
        .execute(
            "INSERT INTO items (id, name) VALUES (?1, ?2)",
            (item.id.to_string(), item.name.as_str()),
        )
        .await?;

    Ok((StatusCode::CREATED, Json(item)))
}

pub async fn get_item(Extension(item): Extension<Item>) -> Json<Item> {
    Json(item)
}

pub async fn patch_item(
    state: State<AppState>,
    Extension(mut item): Extension<Item>,
    Json(item_patch): Json<ItemNameUpdate>,
) -> Result<Json<Item>> {
    item.name = item_patch.name.clone();
    state
        .conn
        .execute(
            "UPDATE items SET name = ?1 WHERE id = ?2;",
            (item_patch.name.as_str(), item.id.to_string()),
        )
        .await?;

    Ok(Json(item))
}

pub async fn delete_item(
    state: State<AppState>,
    Extension(item): Extension<Item>,
) -> Result<Json<Item>> {
    state
        .conn
        .execute("DELETE FROM items WHERE id = ?1;", [item.id.to_string()])
        .await?;

    Ok(Json(item))
}

pub async fn get_item_auctions(
    Extension(item): Extension<Item>,
    state: State<AppState>,
) -> Result<Json<Vec<Auction>>> {
    let auctions = get_item_auctions_libsql_query(&state, &item.id).await?;
    Ok(Json(auctions))
}

pub async fn get_item_auction(Extension(auction): Extension<Auction>) -> Json<Auction> {
    Json(auction)
}

// =========================Middleware=========================
pub async fn middleware_item_exists(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Response {
    let response = get_item_libsql_query(&state, &id).await;
    match response {
        Ok(None) => Error::ItemNotFound.into_response(),
        Err(e) => e.into_response(),
        Ok(Some(item)) => {
            request.extensions_mut().insert(item);
            next.run(request).await
        }
    }
}

pub async fn middleware_item_instance_and_auction_exist(
    state: State<AppState>,
    Path((item_id, auction_id)): Path<(Uuid, Uuid)>,
    mut request: Request,
    next: Next,
) -> Response {
    let response_item = get_item_instance_libsql_query(&state, &item_id).await;
    let response_auction = get_auction_libsql_query(&state, &auction_id).await;

    let item = match response_item {
        Ok(None) => return Error::ItemInstanceNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(item)) => item,
    };
    request.extensions_mut().insert(item);

    let auction = match response_auction {
        Ok(None) => return Error::AuctionNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(auction)) => auction,
    };
    request.extensions_mut().insert(auction);

    next.run(request).await
}
