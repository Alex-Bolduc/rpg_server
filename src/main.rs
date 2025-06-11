use axum::extract::State;
use futures::StreamExt;
use handlers::characters;
use libsql::de::from_row;
use libsql::{Builder, Error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
mod handlers;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Setting up DB
    dotenv::dotenv().ok();
    let db_url = std::env::var("TURSO_DATABASE_URL").expect("TURSO DATABASE URL not set");
    let db_token = std::env::var("TURSO_AUTH_TOKEN").expect("TURSO DATABASE TOKEN not set");

    let db = Builder::new_remote_replica("local.db", db_url, db_token)
        .build()
        .await
        .unwrap();

    let connection = db.connect()?;

    // Creating DB if it doesn't already exist
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

    // Server

    let state = AppState { conn: connection };

    let router = axum::Router::new()
        .route("/characters", axum::routing::get(get_characters))
        .with_state(state);

    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    axum::serve(listener, router).await.unwrap();

    Ok(())
}

#[axum::debug_handler]
async fn get_characters(State(state): State<AppState>) {
    let query = state
        .conn
        .query("SELECT * FROM characters", ())
        .await
        .unwrap();
    let characters: Vec<Character> = into_rows(query).await;
    println!("{:?}", characters);
}

#[derive(Clone)]
struct AppState {
    conn: libsql::Connection,
}

#[derive(Debug, Serialize, Deserialize)]
struct Character {
    name: String,
    class: Class,
    gold: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Class {
    Warrior,
    Mage,
    Ranger,
}

pub async fn into_rows<T>(rows: libsql::Rows) -> Vec<T>
where
    for<'de> T: Deserialize<'de>,
{
    let stream = rows.into_stream();

    stream
        .map(|r| from_row::<T>(&r.unwrap()).unwrap())
        .collect::<Vec<_>>()
        .await
}
