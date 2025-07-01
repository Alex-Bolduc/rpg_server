use std::time::Duration;

use axum::{Router, middleware};
use chrono::Utc;
use errors::{Error, Result};
use futures::TryStreamExt;

use libsql::Builder;
use libsql::de::from_row;
use serde::Deserialize;

use handlers::{
    auctions::get_auctions,
    characters::{
        delete_character, get_character, get_characters, middleware_character_exists,
        patch_character, post_character,
    },
    items::{delete_item, get_item, get_items, middleware_item_exists, patch_item, post_item},
};
use tokio::time::sleep;

use crate::handlers::{
    auctions::{get_auction, middleware_auction_exists, post_auction},
    characters::{
        delete_character_auction, delete_character_item_instance, get_character_auction,
        get_character_auctions, get_character_item, get_character_items,
        middleware_character_and_auction_exist, middleware_character_and_item_exist,
        middleware_character_and_item_instance_exist, post_character_auction, post_character_item,
    },
    items::{get_item_auction, get_item_auctions, middleware_item_instance_and_auction_exist},
};
mod errors;
mod handlers;

#[tokio::main]
async fn main() -> Result<()> {
    // Setting up DB
    dotenv::dotenv().ok();
    let db_url = std::env::var("TURSO_DATABASE_URL").expect("TURSO DATABASE URL not set");
    let db_token = std::env::var("TURSO_AUTH_TOKEN").expect("TURSO DATABASE TOKEN not set");

    let db = Builder::new_remote_replica("local.db", db_url, db_token)
        .build()
        .await
        .unwrap();

    let connection = db.connect()?;

    let state = AppState { conn: connection };

    // Creating characters DB if it doesn't already exist
    state
        .conn
        .execute(
            "CREATE TABLE IF NOT EXISTS characters (
        name TEXT PRIMARY KEY,
        class TEXT NOT NULL CHECK (class IN ('warrior', 'mage', 'ranger')),
        gold INTEGER NOT NULL CHECK (gold >= 0)
        )",
            (),
        )
        .await?;

    // Creating items DB if it doesn't already exist
    state
        .conn
        .execute(
            "CREATE TABLE IF NOT EXISTS items (
        id TEXT PRIMARY KEY CHECK (length(id) = 36),
        name TEXT NOT NULL UNIQUE
        )",
            (),
        )
        .await?;

    // Creating items_instances DB if it doesn't already exist
    state
        .conn
        .execute(
            "CREATE TABLE IF NOT EXISTS items_instances (
        id TEXT PRIMARY KEY CHECK (length(id) = 36),
        item_name TEXT NOT NULL,
        item_id TEXT NOT NULL CHECK (length(item_id) = 36),
        owner_name TEXT NOT NULL,
        FOREIGN KEY (item_name) REFERENCES items(name) ON DELETE CASCADE ON UPDATE CASCADE,
        FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
        FOREIGN KEY (owner_name) REFERENCES characters(name) ON DELETE CASCADE
        )",
            (),
        )
        .await?;

    // Creating auctions DB if it doesn't already exist
    state
        .conn
        .execute(
            "CREATE TABLE IF NOT EXISTS auctions (
        id TEXT PRIMARY KEY CHECK (length(id) = 36),
        auctioned_item_id TEXT NOT NULL CHECK (length(item_auctioned_id) = 36),
        seller_name TEXT NOT NULL,
        creation_date TEXT NOT NULL, 
        end_date TEXT NOT NULL, 
        price INTEGER NOT NULL CHECK (price >= 0), 
        status TEXT NOT NULL CHECK (status IN ('active', 'sold', 'expired')), 
        FOREIGN KEY (auctioned_item_id) REFERENCES items(id) ON DELETE CASCADE,
        FOREIGN KEY (seller_name) REFERENCES characters(name) ON DELETE CASCADE
        )",
            (),
        )
        .await?;

    // Characters router
    let characters = axum::Router::new().route(
        "/characters",
        axum::routing::get(get_characters).post(post_character),
    );

    let characters_name = axum::Router::new()
        .route(
            "/characters/{name}",
            axum::routing::get(get_character)
                .patch(patch_character)
                .delete(delete_character),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_character_exists,
        ));

    let characters_name_items = axum::Router::new()
        .route(
            "/characters/{name}/items",
            axum::routing::get(get_character_items),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_character_exists,
        ));

    let characters_name_items_item_id = axum::Router::new()
        .route(
            "/characters/{name}/items/{item_id}",
            axum::routing::get(get_character_item).delete(delete_character_item_instance),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_character_and_item_instance_exist,
        ));

    let characters_name_items_item_id_post = axum::Router::new()
        .route(
            "/characters/{name}/items/{item_id}",
            axum::routing::post(post_character_item),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_character_and_item_exist,
        ));

    let characters_name_auctions = axum::Router::new()
        .route(
            "/characters/{name}/auctions",
            axum::routing::get(get_character_auctions).post(post_character_auction),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_character_exists,
        ));

    let characters_name_auctions_id = axum::Router::new()
        .route(
            "/characters/{name}/auctions/{id}",
            axum::routing::get(get_character_auction).delete(delete_character_auction),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_character_and_auction_exist,
        ));

    // Items router
    let items = axum::Router::new().route("/items", axum::routing::get(get_items).post(post_item));

    let items_id = axum::Router::new()
        .route(
            "/items/{id}",
            axum::routing::get(get_item)
                .patch(patch_item)
                .delete(delete_item),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_item_exists,
        ));

    let items_id_auctions = axum::Router::new()
        .route(
            "/items/{id}/auctions",
            axum::routing::get(get_item_auctions),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_item_exists,
        ));

    let items_id_auctions_auction_id = axum::Router::new()
        .route(
            "/items/{id}/auctions/{auction_id}",
            axum::routing::get(get_item_auction),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_item_instance_and_auction_exist,
        ));

    // Auctions router
    let auctions = axum::Router::new().route("/auctions", axum::routing::get(get_auctions));

    let auctions_id = axum::Router::new()
        .route("/auctions/{id}", axum::routing::get(get_auction))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_auction_exists,
        ));

    let auctions_id_purchase = axum::Router::new()
        .route("/auctions/{id}/purchase", axum::routing::post(post_auction))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_auction_exists,
        ));

    // Main router (all routers merged)
    let router = Router::new()
        .merge(characters)
        .merge(characters_name)
        .merge(characters_name_items)
        .merge(characters_name_items_item_id)
        .merge(characters_name_items_item_id_post)
        .merge(characters_name_auctions)
        .merge(characters_name_auctions_id)
        .merge(items)
        .merge(items_id)
        .merge(items_id_auctions)
        .merge(items_id_auctions_auction_id)
        .merge(auctions)
        .merge(auctions_id)
        .merge(auctions_id_purchase)
        .with_state(state.clone());
    let address: &'static str = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    spawn_auction_status_updater(state.conn);

    db.sync().await?;
    axum::serve(listener, router).await.unwrap();

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub conn: libsql::Connection,
}

pub async fn into_rows<T>(rows: libsql::Rows) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let items = rows
        .into_stream()
        .map_err(Error::from)
        .and_then(|r| async move { from_row::<T>(&r).map_err(Error::from) })
        .try_collect::<Vec<_>>()
        .await?;
    Ok(items)
}

fn spawn_auction_status_updater(conn: libsql::Connection) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(30)).await;

            let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let result = conn
                .execute(
                    "UPDATE auctions SET status = 'expired' WHERE end_date < ?1 AND status = 'active'",
                    [now],
                )
                .await;

            match result {
                Ok(_) => println!("Auction statuses updated."),
                Err(e) => println!("Failed to update auction statuses: {}", e),
            }
        }
    });
}
