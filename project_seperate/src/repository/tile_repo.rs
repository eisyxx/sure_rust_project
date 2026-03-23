use rusqlite::{Connection, OptionalExtension};

// 지정 토지의 정보를 불러오기
pub fn get_tile_info(
    conn: &Connection,
    tile_id: i32,
) -> rusqlite::Result<(i32, i32, Option<i32>, String)> {
    conn.query_row(
        "SELECT t.price, t.toll, p.owner_id, t.type
         FROM tiles t
         LEFT JOIN properties p ON t.id = p.tile_id
         WHERE t.id = ?1",
        [tile_id],
        |row| {
            Ok((
                row.get::<_, i32>(0)?,         // tiles.price 명시
                row.get::<_, i32>(1)?,                  // toll
                row.get::<_, Option<i32>>(2)?, // owner_id
                row.get::<_, String>(3)?,      // type
            ))
        },
    )
}
