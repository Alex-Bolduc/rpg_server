use libsql::Builder;

fn main() {
    println!("Hello, world!");

    let url = std::env::var("TURSO_DATABASE_URL").expect("TURSO_DATABASE_URL must be set");
    let token = std::env::var("TURSO_AUTH_TOKEN").expect("TURSO_AUTH_TOKEN must be set");
}
