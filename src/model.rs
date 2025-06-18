pub struct Model {
    pub conn: libsql::Connection,
}

impl Model {
    pub fn new_conn(conn: libsql::Connection) -> Self {
        Self { conn }
    }
}
