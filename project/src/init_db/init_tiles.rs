use rusqlite::{Connection, Result};

pub fn init_tiles(conn: &Connection) -> Result<()> {
    // 초기화
    conn.execute("DELETE FROM tiles", [])?;
    conn.execute("DELETE FROM event_tiles", [])?;

    // --- 시작 ---
    conn.execute(
        "INSERT INTO tiles (id, name, type, price)
         VALUES (1, '시작/끝', 'start', 0)",
        [],
    )?;

    let mut id = 2;

    // --- 5 ~ 9 ---
    for price in [5, 6, 7, 8, 9] {
        insert_property(conn,id, price)?;
        id += 1;
    }

    // --- 이벤트 A ---
    insert_event(conn, id, "사회복지기금", "fund_add", 100000)?;
    id += 1;

    // --- 15 ~ 35 ---
    for price in [15, 20, 25, 30, 35] {
        insert_property(conn, id, price)?;
        id += 1;
    }

    // --- 이벤트 B ---
    insert_event(conn, id, "종부세", "tax_if_property", 300000)?;
    id += 1;

    // --- 50 ~ 90 ---
    for price in [50, 60, 70, 80, 90] {
        insert_property(conn, id, price)?;
        id += 1;
    }

    // --- 이벤트 C ---
    insert_event(conn, id, "기금수령", "fund_take", 0)?;
    id += 1;

    // --- 100 ~ 300 ---
    for price in [100, 150, 200, 250, 300] {
        insert_property(conn, id, price)?;
        id += 1;
    }

    println!("맵 초기화 완료");
    Ok(())
}

fn insert_property(conn: &Connection, id: i32, price: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO tiles (id, name, type, price)
         VALUES (?1, ?2, 'property', ?3)",
        (id, format!("토지{}", id), price),
    )?;

    conn.execute(
        "INSERT INTO properties (tile_id, owner_id, price)
         VALUES (?1, NULL, ?2)",
        (id, price),
    )?;

    Ok(())
}

fn insert_event(
    conn: &Connection,
    id: i32,
    name: &str,
    event_type: &str,
    amount: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO tiles (id, name, type, price)
         VALUES (?1, ?2, 'event', 0)",
        (id, name),
    )?;

    conn.execute(
        "INSERT INTO event_tiles (tile_id, event_type, amount, description)
         VALUES (?1, ?2, ?3, ?4)",
        (id, event_type, amount, name),
    )?;

    Ok(())
}