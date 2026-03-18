use rusqlite::{Connection, Result};

mod init_db;

use init_db::create_db::init_db;
use init_db::init_player::create_player;
use init_db::init_tiles::init_tiles;

fn main() -> Result<()> {
    // SQLite 엔진 로드
    let conn = Connection::open("game.db")?;

    // DB 초기화 (테이블 생성)
    init_db(&conn)?;

    // 게임 초기 상태 생성
    conn.execute(
        "INSERT OR IGNORE INTO games(current_turn, status)
         VALUES (1,'playing')",
        [],
    )?;

    // 맵 초기화
    init_tiles(&conn)?;

    // 플레이어 4명 생성
    create_player(&conn, 1, "Player1", 1)?;
    create_player(&conn, 1, "Player2", 2)?;
    create_player(&conn, 1, "Player3", 3)?;
    create_player(&conn, 1, "Player4", 4)?;

    Ok(())
}