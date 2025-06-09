use futures::StreamExt;
use libsql::de::from_row;
use libsql::{Builder, Error};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();

    let router = axum::Router::new().route("/", axum::routing::get(get_handler));

    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    let db_url = std::env::var("TURSO_DATABASE_URL").expect("TURSO DATABASE URL not set");
    let db_token = std::env::var("TURSO_AUTH_TOKEN").expect("TURSO DATABASE TOKEN not set");

    let mut db = Builder::new_remote_replica("local.db", db_url, db_token)
        .build()
        .await
        .unwrap();

    let conn = db.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS characters (
        name TEXT PRIMARY KEY,
        class TEXT NOT NULL CHECK (class IN ('warrior', 'mage', 'ranger')),
        gold INTEGER NOT NULL CHECK (gold >= 0)
        )",
        (),
    )
    .await?;

    let mut test = conn.query("SELECT * FROM characters", ()).await.unwrap();
    let characters: Vec<Character> = into_rows(test).await;
    println!("{:?}", characters);

    axum::serve(listener, router).await.unwrap();

    Ok(())
}

async fn get_handler() {
    println!("Hello World!")
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
