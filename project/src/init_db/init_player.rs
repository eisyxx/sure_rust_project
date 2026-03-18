use rusqlite::{Connection, Result};

pub fn create_player(conn: &Connection, game_id: i32, name: &str, turn_order: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO players (game_id, name, position, money, lap, turn_order, is_bankrupt)
         VALUES (?1, ?2, 0, 3000000, 0, ?3, 0)",
        (game_id, name, turn_order),
    )?;

    let player_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, created_at)
        VALUES (?1, 'deposit', 3000000, '초기자금', datetime('now', '+9 hours'))",
        (player_id,),
    )?;

    println!("플레이어 생성 완료");
    Ok(())
}