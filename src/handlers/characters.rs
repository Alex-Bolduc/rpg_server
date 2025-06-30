use crate::{
    AppState,
    errors::{Error, Result},
    handlers::{
        auctions::{Auction, AuctionStatus, get_auction_libsql_query},
        items::{Item, ItemInstance, get_item_libsql_query},
    },
    into_rows,
};
use axum::{
    extract::{Extension, Json, Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{TimeDelta, Utc};
use libsql::de::from_row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Character {
    pub name: String,
    class: Class,
    pub gold: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharacterGoldUpdate {
    gold: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Class {
    Warrior,
    Mage,
    Ranger,
}

impl Class {
    fn to_string(&self) -> String {
        match self {
            Class::Warrior => "warrior".to_string(),
            Class::Mage => "mage".to_string(),
            Class::Ranger => "ranger".to_string(),
        }
    }
}

// =========================Query functions=========================
async fn get_characters_libsql_query(state: &State<AppState>) -> Result<Vec<Character>> {
    let query = state.conn.query("SELECT * FROM characters", ()).await?;
    let characters: Vec<Character> = into_rows(query).await?;
    Ok(characters)
}

pub async fn get_character_libsql_query(
    state: &State<AppState>,
    name: &String,
) -> Result<Option<Character>> {
    let mut query = state
        .conn
        .query(
            "SELECT * FROM characters WHERE name = ?1",
            [name.to_string()],
        )
        .await?;
    let character = query.next().await?; //None if there are no more rows
    character
        .map(|row| from_row(&row).map_err(Error::from))
        .transpose()
}

async fn get_character_items_libsql_query(
    state: &State<AppState>,
    name: &String,
) -> Result<Vec<ItemInstance>> {
    let query = state
        .conn
        .query(
            "SELECT * FROM items_instances WHERE owner_name = ?1",
            [name.to_string()],
        )
        .await?;
    let items: Vec<ItemInstance> = into_rows(query).await?;
    Ok(items)
}

async fn get_character_item_libsql_query(
    state: &State<AppState>,
    name: &String,
    id: &Uuid,
) -> Result<Option<ItemInstance>> {
    let mut query = state
        .conn
        .query(
            "SELECT * FROM items_instances WHERE owner_name = ?1 AND id = ?2",
            (name.to_string(), id.to_string()),
        )
        .await?;
    let item = query.next().await?; //None if there are no more rows
    item.map(|row| from_row(&row).map_err(Error::from))
        .transpose()
}

async fn get_character_auctions_libsql_query(
    state: &State<AppState>,
    name: &String,
) -> Result<Vec<Auction>> {
    let query = state
        .conn
        .query(
            "SELECT * FROM auctions WHERE seller_name = ?1",
            [name.to_string()],
        )
        .await?;
    let auctions: Vec<Auction> = into_rows(query).await?;
    Ok(auctions)
}

// =========================Handlers=========================
pub async fn get_characters(state: State<AppState>) -> Result<Json<Vec<Character>>> {
    let characters = get_characters_libsql_query(&state).await?;
    Ok(Json(characters))
    // let mut header = HeaderMap::new();
    // header.insert(
    //     CONTENT_TYPE,
    //     "application/json".parse::<HeaderValue>().unwrap(),
    // );
    // (header, serde_json::to_string(&characters).unwrap())
}

pub async fn post_character(
    state: State<AppState>,
    Json(character): Json<Character>,
) -> Result<(StatusCode, Json<Character>)> {
    if character.name.is_empty() {
        return Err(Error::EmptyName);
    }
    state
        .conn
        .execute(
            "INSERT INTO characters (name, class, gold) VALUES (?1, ?2, ?3)",
            (
                character.name.as_str(),
                character.class.to_string(),
                character.gold,
            ),
        )
        .await?;

    Ok((StatusCode::CREATED, Json(character)))
}

pub async fn get_character(Extension(character): Extension<Character>) -> Json<Character> {
    Json(character)
}

pub async fn patch_character(
    state: State<AppState>,
    Extension(mut character): Extension<Character>,
    Json(character_patch): Json<CharacterGoldUpdate>,
) -> Result<Json<Character>> {
    character.gold = character_patch.gold;
    state
        .conn
        .execute(
            "UPDATE characters SET gold = ?1 WHERE name = ?2",
            (character_patch.gold, character.clone().name),
        )
        .await?;

    Ok(Json(character))
}

pub async fn delete_character(
    state: State<AppState>,
    Extension(character): Extension<Character>,
) -> Result<Json<Character>> {
    state
        .conn
        .execute(
            "DELETE FROM characters WHERE name = ?1;",
            [character.clone().name],
        )
        .await?;

    Ok(Json(character))
}

pub async fn get_character_items(
    Extension(character): Extension<Character>,
    state: State<AppState>,
) -> Result<Json<Vec<ItemInstance>>> {
    let items = get_character_items_libsql_query(&state, &character.name).await?;
    Ok(Json(items))
}

pub async fn get_character_item(Extension(item): Extension<Item>) -> Json<Item> {
    Json(item)
}

pub async fn post_character_item(
    state: State<AppState>,
    Extension(character): Extension<Character>,
    Extension(item): Extension<Item>,
) -> Result<(StatusCode, Json<ItemInstance>)> {
    let new_id = Uuid::new_v4();
    let new_item_instance = ItemInstance {
        id: new_id,
        item_name: item.name,
        item_id: item.id,
        owner_name: character.name,
    };

    state
        .conn
        .execute(
            "INSERT INTO items_instances (id, item_name, item_id, owner_name) VALUES (?1, ?2, ?3, ?4)",
            (
                new_item_instance.id.to_string(),
                new_item_instance.item_name.as_str(),
                new_item_instance.item_id.to_string(),
                new_item_instance.owner_name.as_str(),
            ),
        )
        .await?;

    Ok((StatusCode::CREATED, Json(new_item_instance)))
}

pub async fn delete_character_item(
    state: State<AppState>,
    Extension(item): Extension<ItemInstance>,
) -> Result<Json<ItemInstance>> {
    state
        .conn
        .execute(
            "DELETE FROM items_instances WHERE id = ?1",
            [item.id.to_string()],
        )
        .await?;

    Ok(Json(item))
}

pub async fn get_character_auctions(
    Extension(character): Extension<Character>,
    state: State<AppState>,
) -> Result<Json<Vec<Auction>>> {
    let auctions = get_character_auctions_libsql_query(&state, &character.name).await?;
    Ok(Json(auctions))
}

pub async fn get_character_auction(Extension(item): Extension<Item>) -> Json<Item> {
    Json(item)
}

pub async fn post_character_auction(
    state: State<AppState>,
    Extension(character): Extension<Character>,
    Json(item): Json<ItemInstance>,
) -> Result<(StatusCode, Option<Json<Auction>>)> {
    let Some(_) = get_character_item_libsql_query(&state, &character.name, &item.id).await? else {
        return Err(Error::ItemInstanceNotFound);
    };

    let new_id = Uuid::new_v4();
    let new_creation_date = Utc::now();
    let new_end_date = new_creation_date + TimeDelta::minutes(1);
    let new_auction = Auction {
        id: new_id,
        auctioned_item_id: item.item_id,
        seller_name: character.name,
        creation_date: new_creation_date,
        end_date: new_end_date,
        price: 100,
        status: AuctionStatus::Active,
    };

    state
        .conn
        .execute(
            "INSERT INTO auctions (id, auctioned_item_id, seller_name, creation_date, end_date, price, status) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (new_auction.id.to_string(), new_auction.auctioned_item_id.to_string(), new_auction.seller_name.as_str(), new_auction.creation_date.format("%Y-%m-%d %H:%M:%S").to_string(), new_auction.end_date.format("%Y-%m-%d %H:%M:%S").to_string(), new_auction.price, new_auction.status.to_string()),
        )
        .await?;

    Ok((StatusCode::CREATED, Some(Json(new_auction))))
}

pub async fn delete_character_auction(
    state: State<AppState>,
    Extension(auction): Extension<Auction>,
) -> Result<Json<Auction>> {
    state
        .conn
        .execute(
            "DELETE FROM auctions WHERE id = ?1",
            [auction.id.to_string()],
        )
        .await?;

    Ok(Json(auction))
}

// =========================Middleware=========================
pub async fn middleware_character_exists(
    state: State<AppState>,
    Path(name): Path<String>,
    mut request: Request,
    next: Next,
) -> Response {
    let response = get_character_libsql_query(&state, &name).await;
    match response {
        Ok(None) => Error::CharacterNotFound.into_response(),
        Err(e) => e.into_response(),
        Ok(Some(character)) => {
            request.extensions_mut().insert(character);
            next.run(request).await
        }
    }
}

pub async fn middleware_character_and_item_exist(
    state: State<AppState>,
    Path((name, id)): Path<(String, Uuid)>,
    mut request: Request,
    next: Next,
) -> Response {
    let response_character = get_character_libsql_query(&state, &name).await;
    let response_item = get_item_libsql_query(&state, &id).await;

    let character = match response_character {
        Ok(None) => return Error::CharacterNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(character)) => character,
    };
    request.extensions_mut().insert(character);

    let item = match response_item {
        Ok(None) => return Error::ItemNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(item)) => item,
    };
    request.extensions_mut().insert(item);

    next.run(request).await
}

pub async fn middleware_character_and_item_instance_exist(
    state: State<AppState>,
    Path((name, id)): Path<(String, Uuid)>,
    mut request: Request,
    next: Next,
) -> Response {
    let response_character = get_character_libsql_query(&state, &name).await;
    let response_item = get_character_item_libsql_query(&state, &name, &id).await;

    let character = match response_character {
        Ok(None) => return Error::CharacterNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(character)) => character,
    };
    request.extensions_mut().insert(character);

    let item = match response_item {
        Ok(None) => return Error::ItemInstanceNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(item)) => item,
    };
    request.extensions_mut().insert(item);

    next.run(request).await
}

pub async fn middleware_character_and_auction_exist(
    state: State<AppState>,
    Path((name, id)): Path<(String, Uuid)>,
    mut request: Request,
    next: Next,
) -> Response {
    let response_character = get_character_libsql_query(&state, &name).await;
    let response_auction = get_auction_libsql_query(&state, &id).await;

    let character = match response_character {
        Ok(None) => return Error::CharacterNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(character)) => character,
    };
    request.extensions_mut().insert(character);

    let auction = match response_auction {
        Ok(None) => return Error::AuctionNotFound.into_response(),
        Err(e) => return e.into_response(),
        Ok(Some(auction)) => auction,
    };
    request.extensions_mut().insert(auction);

    next.run(request).await
}
