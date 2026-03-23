use rusqlite::{Connection, Result};

// 4명의 플레이어 생성
pub fn create_player(conn: &Connection, game_id: i32, name: &str, turn_order: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO players (game_id, name, position, money, lap, turn_order, is_bankrupt)
         VALUES (?1, ?2, 0, 300, 0, ?3, 0)",
        (game_id, name, turn_order),
    )?;

    let player_id = conn.last_insert_rowid();

    // 초기자금(300만원) 입금
    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, balance_before, balance_after, created_at)
        VALUES (?1, 'deposit', 300, '초기자금', 0, 300, datetime('now', '+9 hours'))",
        (player_id,),
    )?;

    println!("플레이어 생성 완료");
    Ok(())
}