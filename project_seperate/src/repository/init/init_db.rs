use rusqlite::Connection;

use crate::repository::init::create_db::create_db;
use crate::repository::init::init_player::create_player;
use crate::repository::init::init_tiles::init_tiles;

pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    create_db(conn)?;

    conn.execute("DELETE FROM transactions", [])?;
    conn.execute("DELETE FROM properties", [])?;
    conn.execute("DELETE FROM event_tiles", [])?;
    conn.execute("DELETE FROM tiles", [])?;
    conn.execute("DELETE FROM players", [])?;
    conn.execute("DELETE FROM games", [])?;
    conn.execute("DELETE FROM fund", [])?;

    conn.execute(
        "INSERT INTO games (current_turn, status) VALUES (1, 'playing')",
        [],
    )?;
    conn.execute("INSERT INTO fund (amount) VALUES (0)", [])?;

    init_tiles(conn)?;

    create_player(conn, 1, "Player1", 1)?;
    create_player(conn, 1, "Player2", 2)?;
    create_player(conn, 1, "Player3", 3)?;
    create_player(conn, 1, "Player4", 4)?;

    println!("DB 초기화 완료");
    Ok(())
}