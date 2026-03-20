use rusqlite::{Connection, Result, OptionalExtension};

pub fn get_owner(conn: &Connection, tile_id: i32) -> Result<Option<i32>> {
    conn.query_row(
        "SELECT owner_id FROM properties WHERE tile_id = ?1",
        [tile_id],
        |row| row.get::<_, Option<i32>>(0),
    )
    .optional()
    .map(|opt| opt.flatten())
}

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