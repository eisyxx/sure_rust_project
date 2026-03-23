use rusqlite::{Connection, Result, OptionalExtension};

#[derive(Clone)]
pub struct TileOwnerRecord {
    pub tile_id: i32,
    pub owner_id: i32,
}

// 지정 토지의 소유자 ID 불러오기
pub fn get_owner(conn: &Connection, tile_id: i32) -> Result<Option<i32>> {
    conn.query_row(
        "SELECT owner_id FROM properties WHERE tile_id = ?1",
        [tile_id],
        |row| row.get::<_, Option<i32>>(0),
    )
    .optional()
    .map(|opt| opt.flatten())
}

// 지정 토지의 소유자와 가격 업데이트 (oner_id가 NULL인 경우에만 가능)
pub fn set_owner(conn: &Connection, tile_id: i32, owner_id: i32, price: i32) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO properties (tile_id, owner_id, price)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(tile_id) DO UPDATE 
         SET owner_id = excluded.owner_id, price = excluded.price
         WHERE properties.owner_id IS NULL",
        (tile_id, owner_id, price),
    )?;
    Ok(())
}

// 현재 소유자가 있는 모든 토지와 소유자 정보를 반환
pub fn get_owned_tiles(conn: &Connection) -> Result<Vec<TileOwnerRecord>> {
    let mut stmt = conn.prepare(
        "SELECT tile_id, owner_id
         FROM properties
         WHERE owner_id IS NOT NULL"
    )?;

    let records = stmt.query_map([], |row| {
        Ok(TileOwnerRecord {
            tile_id: row.get(0)?,
            owner_id: row.get(1)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

// 지정 플레이어가 소유한 모든 토지의 소유자 정보를 초기화
pub fn reset_owner_for_player(conn: &Connection, player_id: i32) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE properties SET owner_id = NULL WHERE owner_id = ?1",
        [player_id],
    )?;
    Ok(())
}