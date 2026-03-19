use rusqlite::{Connection, Result};

use crate::service::game_end_service::Player;

pub fn get_player_money(conn: &Connection, player_id: i32) -> Result<i32> {
    conn.query_row(
        "SELECT money FROM players WHERE id = ?1",
        [player_id],
        |row| row.get(0),
    )
}

pub fn update_money(conn: &Connection, player_id: i32, delta: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = money + ?1 WHERE id = ?2",
        (delta, player_id),
    )?;
    Ok(())
}

pub fn bankrupt(conn: &Connection, player_id: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = 0, is_bankrupt = 1 WHERE id = ?1",
        [player_id],
    )?;
    Ok(())
}

pub fn get_all_players(conn: &Connection) -> Result<Vec<Player>> {
    let mut stmt = conn.prepare(
        "SELECT id, position, money, lap, is_bankrupt FROM players"
    )?;

    let players = stmt.query_map([], |row| {
        Ok(Player {
            id: row.get(0)?,
            position: row.get(1)?,
            money: row.get(2)?,
            lap: row.get(3)?,
            is_bankrupt: row.get::<_, i32>(4)? == 1,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(players)
}

pub fn update_player_money(conn: &Connection, player_id: i32, money: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = ?1 WHERE id = ?2",
        (money, player_id),
    )?;
    Ok(())
}

pub fn update_position_and_lap(
    conn: &Connection,
    player_id: i32,
    pos: i32,
    lap: i32,
) -> Result<()> {
    conn.execute(
        "UPDATE players SET position = ?1, lap = ?2 WHERE id = ?3",
        (pos, lap, player_id),
    )?;
    Ok(())
}