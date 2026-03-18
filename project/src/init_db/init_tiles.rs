use rusqlite::{Connection, Result};

pub fn init_tiles(conn: &Connection) -> Result<()> {
    // 초기화
    conn.execute("DELETE FROM tiles", [])?;
    conn.execute("DELETE FROM event_tiles", [])?;

    // 시작점 타일을 DB에 추가
    conn.execute(
        "INSERT INTO tiles (id, name, type, price, toll)
         VALUES (1, '시작/끝', 'start', 0, 0)",
        [],
    )?;

    // 토지 및 이벤트 순서대로 price와 toll
    let tile_data = vec![
        // (id, price, name, toll)
        (2, 5, "101", 3),
        (3, 6, "102", 4),
        (4, 7, "103", 4),
        (5, 8, "104", 5),
        (6, 9, "105", 5),
        (7, 0, "사회복지기금", 0),
        (8, 15, "201", 9),
        (9, 20, "202", 12),
        (10, 25, "203", 15),
        (11, 30, "204", 18),
        (12, 35, "205", 21),
        (13, 0, "종부세", 0),
        (14, 50, "301", 30),
        (15, 60, "302", 36),
        (16, 70, "303", 42),
        (17, 80, "304", 48),
        (18, 90, "305", 54),
        (19, 0, "기금수령", 0),
        (20, 100, "401", 60),
        (21, 150, "402", 90),
        (22, 200, "403", 120),
        (23, 250, "404", 150),
        (24, 300, "405", 180),
    ];

    for (id, price, name, toll) in tile_data {
        if price == 0 {
            // 이벤트 칸
            insert_event(conn, id, name, toll)?;
        } else {
            // 토지 칸
            insert_property(conn, id, price, name, toll)?;
        }
    }

    println!("맵 초기화 완료");
    Ok(())
}

// 토지 정보를 DB에 추가
fn insert_property(conn: &Connection, id: i32, price: i32, name: &str, toll: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO tiles (id, name, type, price, toll)
         VALUES (?1, ?2, 'property', ?3, ?4)",
        (id, name, price, toll),
    )?;

    conn.execute(
        "INSERT INTO properties (tile_id, owner_id, price)
         VALUES (?1, NULL, ?2)",
        (id, price),
    )?;

    Ok(())
}

// 이벤트 정보를 DB에 추가
fn insert_event(conn: &Connection, id: i32, name: &str, toll: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO tiles (id, name, type, price, toll)
         VALUES (?1, ?2, 'event', 0, ?3)",
        (id, name, toll),
    )?;

    conn.execute(
        "INSERT INTO event_tiles (tile_id, event_type, amount, description)
         VALUES (?1, ?2, ?3, ?4)",
        (id,
         match name {
             "사회복지기금" => "fund_add",
             "종부세" => "tax_if_property",
             "기금수령" => "fund_take",
             _ => "",
         },
         match name {
             "사회복지기금" => 100_000,
             "종부세" => 300_000,
             "기금수령" => 0,
             _ => 0,
         },
         name),
    )?;

    Ok(())
}