use crate::errors::{Error, Result};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use futures::StreamExt;
use libsql::de::from_row;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub conn: libsql::Connection,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Character {
    name: String,
    class: Class,
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

async fn get_characters_libsql_query(conn: &libsql::Connection) -> Vec<Character> {
    let query = conn.query("SELECT * FROM characters", ()).await.unwrap();
    let characters: Vec<Character> = into_rows(query).await;
    characters
}

async fn get_character_libsql_query(conn: &libsql::Connection, name: String) -> Character {
    let mut query = conn
        .query(
            "SELECT name, class, gold FROM characters WHERE name = ?1",
            [name],
        )
        .await
        .unwrap();
    let character = query.next().await.unwrap().unwrap();
    let character: Character = from_row(&character).unwrap();
    character
}

pub async fn get_characters(State(state): State<AppState>) -> Json<Vec<Character>> {
    let characters = get_characters_libsql_query(&state.conn).await;
    println!("{:?}", characters);
    Json(characters)
    // let mut header = HeaderMap::new();
    // header.insert(
    //     CONTENT_TYPE,
    //     "application/json".parse::<HeaderValue>().unwrap(),
    // );
    // (header, serde_json::to_string(&characters).unwrap())
}

pub async fn post_characters(
    State(state): State<AppState>,
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
        .await
        .unwrap();

    Ok((StatusCode::CREATED, Json(character)))
}

pub async fn get_character(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Character> {
    let character = get_character_libsql_query(&state.conn, name).await;
    println!("{:?}", character);
    Json(character)
}
