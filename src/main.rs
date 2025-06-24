use axum::{Router, middleware};
use errors::{Error, Result};
use futures::TryStreamExt;
use handlers::characters::{
    delete_character, get_character, get_characters, middleware_character_exists, patch_character,
    post_character,
};
use libsql::Builder;
use libsql::de::from_row;
use serde::Deserialize;

use crate::handlers::items::{get_items, post_item};
mod errors;

mod handlers;
mod model;

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

    // Creating characters DB if it doesn't already exist
    connection
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
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS items (
        id TEXT PRIMARY KEY CHECK (length(id) = 36),
        name TEXT NOT NULL
        )",
            (),
        )
        .await?;

    // Server
    let state = AppState { conn: connection };

    let characters_router = axum::Router::new().route(
        "/characters",
        axum::routing::get(get_characters).post(post_character),
    );
    let characters_named_router = axum::Router::new()
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

    let items_router =
        axum::Router::new().route("/items", axum::routing::get(get_items).post(post_item));

    let router = Router::new()
        .merge(characters_router)
        .merge(characters_named_router)
        .merge(items_router)
        .with_state(state);
    let address: &'static str = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

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
