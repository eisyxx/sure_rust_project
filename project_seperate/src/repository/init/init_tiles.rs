use rusqlite::{Connection, Result};

pub fn init_tiles(conn: &Connection) -> Result<()> {
    // 시작점 타일을 DB에 추가
    conn.execute(
        "INSERT INTO tiles (id, name, type, price, toll)
         VALUES (0, '시작/끝', 'start', 0, 0)",
        [],
    )?;

    // 토지 및 이벤트 순서대로 price와 toll
    let tile_data = vec![
        // (id, price, name, toll)
        (1, 5, "101", 3),
        (2, 6, "102", 4),
        (3, 7, "103", 4),
        (4, 8, "104", 5),
        (5, 9, "105", 5),
        (6, 0, "사회복지기금", 0),
        (7, 15, "201", 9),
        (8, 20, "202", 12),
        (9, 25, "203", 15),
        (10, 30, "204", 18),
        (11, 35, "205", 21),
        (12, 0, "종부세", 0),
        (13, 50, "301", 30),
        (14, 60, "302", 36),
        (15, 70, "303", 42),
        (16, 80, "304", 48),
        (17, 90, "305", 54),
        (18, 0, "기금수령", 0),
        (29, 100, "401", 60),
        (20, 150, "402", 90),
        (21, 200, "403", 120),
        (22, 250, "404", 150),
        (23, 300, "405", 180),
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
             "사회복지기금" => 10,
             "종부세" => 30,
             "기금수령" => 0,
             _ => 0,
         },
         name),
    )?;

    Ok(())
}