use rusqlite::{Connection, Result};

#[derive(Clone)]
pub struct TransactionRecord {
    pub id: i32,
    pub tx_type: String,
    pub amount: i32,
    pub target: String,
    pub created_at: String,
}

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

pub fn get_transactions_by_player(conn: &Connection, player_id: i32) -> Result<Vec<TransactionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, amount, target, created_at
         FROM transactions
         WHERE player_id = ?1
         ORDER BY id DESC"
    )?;

    let transactions = stmt.query_map([player_id], |row| {
        Ok(TransactionRecord {
            id: row.get(0)?,
            tx_type: row.get(1)?,
            amount: row.get(2)?,
            target: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(transactions)
}