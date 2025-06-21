use crate::errors::{Error, Result};
use axum::{
    extract::{Extension, Json, Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use libsql::de::from_row;
use serde::{Deserialize, Serialize};

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

#[derive(Clone)]
pub struct AppState {
    pub conn: libsql::Connection,
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

// =========================Query functions=========================
async fn get_characters_libsql_query(state: &State<AppState>) -> Result<Vec<Character>> {
    let query = state.conn.query("SELECT * FROM characters", ()).await?;
    let characters: Vec<Character> = into_rows(query).await?;
    Ok(characters)
}

async fn get_character_libsql_query(
    state: &State<AppState>,
    name: &String,
) -> Result<Option<Character>> {
    let mut query = state
        .conn
        .query(
            "SELECT name, class, gold FROM characters WHERE name = ?1",
            [name.to_string()],
        )
        .await?;
    let character = query.next().await?; //None if there are no more rows
    character
        .map(|row| from_row(&row).map_err(Error::from))
        .transpose()
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
    Json(character_patch): Json<Character>,
) -> Result<Json<Character>> {
    character.gold = character_patch.gold;
    state
        .conn
        .execute(
            "UPDATE characters SET gold = ?1 WHERE name = ?2;",
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

// =========================Middleware=========================
pub async fn middleware_character_exists(
    state: State<AppState>,
    Path(name): Path<String>,
    mut request: Request,
    next: Next,
) -> Response {
    let response = get_character_libsql_query(&state, &name).await;
    match response {
        Ok(None) => {
            return Error::CharacterNotFound.into_response();
        }
        Err(e) => {
            return e.into_response();
        }
        Ok(Some(character)) => {
            request.extensions_mut().insert(character);
            next.run(request).await
        }
    }
}
