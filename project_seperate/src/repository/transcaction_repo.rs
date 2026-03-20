use rusqlite::{Connection, Result};

pub fn record_transaction(
    conn: &Connection,
    player_id: i32,
    tx_type: &str,   // "deposit" | "withdraw"
    amount: i32,
    target: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now','localtime'))",
        (player_id, tx_type, amount, target),
    )?;
    Ok(())
}