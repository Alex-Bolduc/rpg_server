use libsql::{Builder, Error};

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

    // conn.execute(
    //     "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
    //     (),
    // )
    // .await?;

    axum::serve(listener, router).await.unwrap();

    Ok(())
}

async fn get_handler() {
    println!("Hello World!")
}
