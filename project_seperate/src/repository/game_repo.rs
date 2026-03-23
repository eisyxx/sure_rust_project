use rusqlite::{Connection, Result};

// 현재 순서인 플레이어 ID를 가져오기
pub fn get_current_turn(conn: &Connection) -> Result<i32> {
    conn.query_row(
        "SELECT current_turn FROM games LIMIT 1",
        [],
        |row| row.get(0),
    )
}

// 현재 턴 순서를 업데이트
pub fn update_current_turn(conn: &Connection, turn: i32) -> Result<()> {
    conn.execute(
        "UPDATE games SET current_turn = ?1",
        [turn],
    )?;
    Ok(())
}

// 현재 게임 상태(playing, finished) 불러오기
pub fn get_game_status(conn: &Connection) -> Result<String> {
    conn.query_row(
        "SELECT status FROM games LIMIT 1",
        [],
        |row| row.get(0),
    )
}

// 게임 상태를 지정 값으로(playing, finished) 설정하기
pub fn set_game_status(conn: &Connection, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE games SET status = ?1",
        [status],
    )?;
    Ok(())
}

// 게임 상태를 finished로 변경
pub fn set_game_finished(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE games SET status = 'finished'",
        [],
    )?;
    Ok(())
}