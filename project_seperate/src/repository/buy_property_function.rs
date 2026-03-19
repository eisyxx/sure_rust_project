use rusqlite::{Connection, Result};

pub fn get_player_money(conn: &Connection, player_id: i32) -> Result<i32> {
    conn.query_row(
        "SELECT money FROM players WHERE id = ?1",
        [player_id],
        |row| row.get(0),
    )
}

pub fn update_money(conn: &Connection, player_id: i32, amount: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = money + ?1 WHERE id = ?2",
        (amount, player_id),
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

pub fn get_tile_info(conn: &Connection, tile_id: i32) -> Result<(i32, i32)> {
    conn.query_row(
        "SELECT price, toll FROM tiles WHERE id = ?1",
        [tile_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

pub fn get_owner(conn: &Connection, tile_id: i32) -> Result<Option<i32>> {
    conn.query_row(
        "SELECT owner_id FROM properties WHERE tile_id = ?1",
        [tile_id],
        |row| row.get::<_, Option<i32>>(0),
    )
    .optional()
    .map(|opt| opt.flatten())
}

pub fn set_owner(conn: &Connection, tile_id: i32, player_id: i32, price: i32) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO properties (tile_id, owner_id, price)
         VALUES (?1, ?2, ?3)",
        (tile_id, player_id, price),
    )?;
    Ok(())
}

pub fn record_transaction(
    conn: &Connection,
    player_id: i32,
    amount: i32,
    target: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, created_at)
         VALUES (?1, 'withdraw', ?2, ?3, datetime('now','localtime'))",
        (player_id, amount, target),
    )?;
    Ok(())
}