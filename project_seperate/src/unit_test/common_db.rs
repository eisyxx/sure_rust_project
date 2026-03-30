use rusqlite::Connection;

pub fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    // 필요한 테이블 전부 생성
    conn.execute(
        "CREATE TABLE players (id INTEGER, money INTEGER)",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE events (tile_id INTEGER, type TEXT, amount INTEGER)",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE properties (tile_id INTEGER, owner_id INTEGER)",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE fund (amount INTEGER)",
        [],
    ).unwrap();

    conn
}
