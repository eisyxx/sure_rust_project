use rusqlite::Connection;

use crate::repository::init::create_db::create_db;
use crate::repository::init::init_player::create_player;
use crate::repository::init::init_tiles::init_tiles;

pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    create_db(conn)?;
    init_tiles(conn)?;

    create_player(conn, 1, "Player1", 1)?;
    create_player(conn, 2, "Player2", 2)?;
    create_player(conn, 3, "Player3", 3)?;
    create_player(conn, 4, "Player4", 4)?;

    println!("DB 초기화 완료");
    Ok(())
}