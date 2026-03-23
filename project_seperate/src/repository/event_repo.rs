use rusqlite::{Connection, Result};

/// 이벤트 정보 조회
pub fn get_event_info(
    conn: &Connection,
    tile_id: i32,
) -> Result<(String, i32)> {
    conn.query_row(
        "SELECT event_type, amount
         FROM event_tiles
         WHERE tile_id = ?1",
        [tile_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?, // event_type
                row.get::<_, i32>(1)?,    // amount
            ))
        },
    )
}

/// 사회복지기금 증가
pub fn add_fund(conn: &Connection, amount: i32) -> Result<()> {
    conn.execute(
        "UPDATE fund SET amount = amount + ?1",
        [amount],
    )?;
    Ok(())
}

/// 현재 기금 조회
pub fn get_fund_amount(conn: &Connection) -> Result<i32> {
    conn.query_row(
        "SELECT amount FROM fund",
        [],
        |row| row.get(0),
    )
}

/// 기금 초기화 (0으로)
pub fn reset_fund(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE fund SET amount = 0",
        [],
    )?;
    Ok(())
}