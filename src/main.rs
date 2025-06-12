use handlers::characters::{AppState, get_characters, post_characters};
use libsql::{Builder, Error};

use crate::handlers::characters::get_character;
mod errors;
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

    let characters_router = axum::Router::new()
        .route(
            "/characters",
            axum::routing::get(get_characters).post(post_characters),
        )
        .route("/characters/{name}", axum::routing::get(get_character))
        .with_state(state);

    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    db.sync().await?;
    axum::serve(listener, characters_router).await.unwrap();

    Ok(())
}
