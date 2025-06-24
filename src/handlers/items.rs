use crate::{
    AppState,
    errors::{Error, Result},
    handlers::characters::Character,
    into_rows,
};
use axum::{
    extract::{Extension, Json, Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use libsql::de::from_row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    id: Uuid,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewItem {
    name: String,
}

// =========================Query functions=========================
async fn get_items_libsql_query(state: &State<AppState>) -> Result<Vec<Item>> {
    let query = state.conn.query("SELECT * FROM items", ()).await?;
    let items: Vec<Item> = into_rows(query).await?;
    Ok(items)
}

async fn get_item_libsql_query(state: &State<AppState>, id: &Uuid) -> Result<Option<Item>> {
    let mut query = state
        .conn
        .query("SELECT id, name FROM items WHERE id = ?1", [id.to_string()])
        .await?;
    let item = query.next().await?; //None if there are no more rows
    item.map(|row| from_row(&row).map_err(Error::from))
        .transpose()
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

    let id = Uuid::new_v4();
    let item = Item {
        id: id,
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

// =========================Middleware=========================
pub async fn middleware_item_exists(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Response {
    let response = get_item_libsql_query(&state, &id).await;
    match response {
        Ok(None) => {
            return Error::ItemNotFound.into_response();
        }
        Err(e) => {
            return e.into_response();
        }
        Ok(Some(item)) => {
            request.extensions_mut().insert(item);
            next.run(request).await
        }
    }
}
