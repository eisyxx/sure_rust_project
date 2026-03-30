use rusqlite::{Connection, Result};
use serde::Serialize;

// 플레이어의 거래 내역을 나타내는 구조체
#[derive(Clone, Serialize)]
pub struct TransactionRecord {
    pub id: i32,
    pub tx_type: String,
    pub amount: i32,
    pub target: String,
    pub balance_before: i32,
    pub balance_after: i32,
    pub created_at: String,
}

// 지정 플레이어의 거래 내역을 DB에 기록
pub fn record_transaction(
    conn: &Connection,
    player_id: i32,
    tx_type: &str,   // "deposit" | "withdraw"
    amount: i32,
    target: &str,
) -> Result<()> {
    // 현재 플레이어의 잔액을 조회
    let mut stmt = conn.prepare(
        "SELECT money FROM players WHERE id = ?1"
    )?;
    
    let balance_after: i32 = stmt.query_row([player_id], |row| {
        row.get(0)
    })?;
    
    // 거래 전 잔액 계산
    let balance_before = if tx_type == "deposit" {
        balance_after - amount
    } else if tx_type == "withdraw" {
        balance_after + amount
    } else {
        balance_after
    };
    
    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, balance_before, balance_after, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now','localtime'))",
        (player_id, tx_type, amount, target, balance_before, balance_after),
    )?;
    Ok(())
}

// 지정 플레이어의 거래 내역 조회
pub fn get_transactions_by_player(conn: &Connection, player_id: i32) -> Result<Vec<TransactionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id,
                type,
                amount,
                target,
                COALESCE(balance_before, 0) AS balance_before,
                COALESCE(balance_after, 0) AS balance_after,
                created_at
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
            balance_before: row.get(4)?,
            balance_after: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(transactions)
}