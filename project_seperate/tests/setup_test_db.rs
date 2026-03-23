use rusqlite::Connection;
use project::repository::init::init_db::init_db;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    init_db(&conn).unwrap(); //매 테스트가 새 DB 생성 후 시작되도록

    conn
}