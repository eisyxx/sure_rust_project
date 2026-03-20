use rusqlite::{Connection, Result};

pub fn get_current_turn(conn: &Connection) -> Result<i32> {
    conn.query_row(
        "SELECT current_turn FROM games LIMIT 1",
        [],
        |row| row.get(0),
    )
}

pub fn update_current_turn(conn: &Connection, turn: i32) -> Result<()> {
    conn.execute(
        "UPDATE games SET current_turn = ?1",
        [turn],
    )?;
    Ok(())
}

pub fn get_game_status(conn: &Connection) -> Result<String> {
    conn.query_row(
        "SELECT status FROM games LIMIT 1",
        [],
        |row| row.get(0),
    )
}

pub fn set_game_status(conn: &Connection, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE games SET status = ?1",
        [status],
    )?;
    Ok(())
}

pub fn set_game_finished(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE games SET status = 'finished'",
        [],
    )?;
    Ok(())
}