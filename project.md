# 📘 RPG Server

## Overview

The project involves writing a REST API that manages characters in a role-playing game, allowing them to own items, participate in auctions, and interact with an in-game economy. The API stores the data in a SQLite database (in this case Turso LibSQL) which is exposed in the multiple different endpoints and their respective Methods.


## Requirements

- The server must use Tokio for asynchronous programming [Docs](https://docs.rs/tokio/latest/tokio/)
- The server must use the web framework [Axum](https://github.com/tokio-rs/axum) [Docs](https://docs.rs/axum/latest/axum/)
- The database service used must be Turso [Platform](https://turso.tech/) (create free account via Github OAuth) [Docs](https://docs.turso.tech/sdk/rust/quickstart)
- The database service must use embedded replicas for fast reads

## 📦 Entities

### 1. Character

#### - Fields:

- `name`: unique identifier for the character (string)

- `class`: character class/type `warrior` | `mage` | `ranger`

- `gold`: the amount of gold the character currently possesses (positive integer)

#### - Routes:

- `GET /characters`: List all characters

- `POST /characters`: Create a new character

- `GET /characters/{name}`: Get a character by name

- `PATCH /characters/{name}`: Update a character’s gold, the class is immutable

- `DELETE /characters/{name}`: Delete a character

#### - Related Resources:

- `GET /characters/{name}/items`: Get all items owned by the character

- `GET /characters/{name}/items/{item_id}`: Get a specific item owned by the character

- `POST /characters/{name}/items/{item_id}`: Add item to character (this endpoint is how to basically add an item to a character, like 'looting' in a game)

- `DELETE /characters/{name}/items/{item_id}`: Remove item from character

- `GET /characters/{name}/auctions`: List all auctions created by the character

- `GET /characters/{name}/auctions/{id}`: Get specific auction created by character

- `POST /characters/{name}/auctions`: Create new auction for an item

- `DELETE /characters/{name}/auctions/{id}`: Cancel auction

- `POST /characters/{name}/auctions/{id}/purchase`: Purchase the auction and associate the item to the buyer (do POST /auctions instead)


### 2. Item

#### - Fields:

`id`: unique identifier (UUID)

`name`: item name

#### - Routes:

`GET /items`: List all item definitions

`POST /items`: Create a new item definition

`GET /items/{id}`: Get specific item definition

`PATCH /items/{id}`: Update an item definition name

`DELETE /items/{id}`: Delete a specific item definition

`GET /items/{id}/auctions`: List all auctions for this item

`GET /items/{id}/auctions/{auction_id}`: Get specific auction for this item

### 3. Auction

#### - Fields:

- `id`: unique auction identifier

- `creation_date`: timestamp

- `end_date`: timestamp

- `price`: price in gold

- `status`: `active` | `sold` | `expired`

#### - Routes:

`GET /auctions`: List all auctions (it must be possible to query actions by status via the querystring)

`GET /auctions/{id}`: Get specific auction

`POST /auctions/{id}/purchase`: Purchase the auction and associate the item to the buyer

## 📜 Business Rules:

### 1. To create an auction:

- The character must own at least one instance of the item.

### 2. To purchase an auction:

- The auction must not be expired.

- The character must have enough gold to pay the price.

- Once purchased, gold is transferred from buyer to seller.

- The item is transferred to the buyer.

- The seller cannot be the buyer.

- The auction status goes from "Active" to "Sold".

- The buyer must exist.

### 3. Expired auctions:

- Auctions' statuses have to be updated automatically without being queried. 

### 4. Items:

- Items are uniquely defined, but multiple instances can exist and be owned by different characters.
