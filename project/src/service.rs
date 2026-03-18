use rand::Rng;
use rusqlite::{Connection, Result};

// =========================
// 🎲 주사위
// =========================
fn roll_dice() -> u8 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=6)
}

// =========================
// 🎯 현재 턴 플레이어 가져오기
// =========================
fn get_current_player(conn: &Connection, game_id: i32) -> Result<(i64, i32, i32)> {
    let mut stmt = conn.prepare(
        "SELECT id, position, lap FROM players
         WHERE game_id = ?1 AND current_turn = 1 AND is_bankrupt = 0"
    )?;

    let player = stmt.query_row((game_id,), |row| {
        Ok((
            row.get(0)?, // id
            row.get(1)?, // position
            row.get(2)?, // lap
        ))
    })?;

    Ok(player)
}

// =========================
// 🧱 보드 크기
// =========================
fn get_board_size(conn: &Connection) -> Result<i32> {
    conn.query_row(
        "SELECT COUNT(*) FROM tiles",
        [],
        |row| row.get(0),
    )
}

// =========================
// 🚶 플레이어 이동
// =========================
fn move_player(conn: &Connection, player_id: i64, dice: u8) -> Result<()> {
    let (position, lap): (i32, i32) = conn.query_row(
        "SELECT position, lap FROM players WHERE id = ?1",
        (player_id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let board_size = get_board_size(conn)?;
    let new_position = position + dice as i32;

    let mut new_lap = lap;

    // 시작칸 통과 체크
    if new_position >= board_size {
        new_lap += 1;

        conn.execute(
            "UPDATE players SET money = money + 20 WHERE id = ?1",
            (player_id,),
        )?;
    }

    let final_position = new_position % board_size;

    conn.execute(
        "UPDATE players
         SET position = ?1, lap = ?2
         WHERE id = ?3",
        (final_position, new_lap, player_id),
    )?;

    Ok(())
}

// =========================
// 🧩 타일 처리
// =========================
fn handle_tile(conn: &Connection, player_id: i64) -> Result<()> {
    let position: i32 = conn.query_row(
        "SELECT position FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    let (tile_type, tile_id): (String, i32) = conn.query_row(
        "SELECT type, id FROM tiles WHERE id = ?1",
        (position,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    match tile_type.as_str() {
        "property" => handle_property(conn, player_id, tile_id)?,
        "event" => handle_event(conn, player_id, tile_id)?,
        _ => {} // start
    }

    Ok(())
}

// =========================
// 🏠 땅 처리
// =========================
fn handle_property(conn: &Connection, player_id: i64, tile_id: i32) -> Result<()> {
    let (owner_id, price): (Option<i64>, i32) = conn.query_row(
        "SELECT owner_id, price FROM properties WHERE tile_id = ?1",
        (tile_id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    match owner_id {
        None => {
            // 구매
            conn.execute(
                "UPDATE properties SET owner_id = ?1 WHERE tile_id = ?2",
                (player_id, tile_id),
            )?;

            conn.execute(
                "UPDATE players SET money = money - ?1 WHERE id = ?2",
                (price, player_id),
            )?;
        }
        Some(owner) if owner != player_id => {
            // 통행료
            let toll = price / 2;

            conn.execute(
                "UPDATE players SET money = money - ?1 WHERE id = ?2",
                (toll, player_id),
            )?;

            conn.execute(
                "UPDATE players SET money = money + ?1 WHERE id = ?2",
                (toll, owner),
            )?;
        }
        _ => {}
    }

    Ok(())
}

// =========================
// 🎁 이벤트 처리
// =========================
fn handle_event(conn: &Connection, player_id: i64, tile_id: i32) -> Result<()> {
    let (event_type, amount): (String, i32) = conn.query_row(
        "SELECT event_type, amount FROM event_tiles WHERE tile_id = ?1",
        (tile_id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    match event_type.as_str() {
        "fund_add" => {
            conn.execute(
                "UPDATE players SET money = money + ?1 WHERE id = ?2",
                (amount, player_id),
            )?;
        }
        "fund_take" => {
            conn.execute(
                "UPDATE players SET money = money + 500000 WHERE id = ?1",
                (player_id,),
            )?;
        }
        "tax_if_property" => {
            conn.execute(
                "UPDATE players SET money = money - ?1 WHERE id = ?2",
                (amount, player_id),
            )?;
        }
        _ => {}
    }

    Ok(())
}

// =========================
// 🔄 턴 넘기기
// =========================
fn next_turn(conn: &Connection, game_id: i32) -> Result<()> {
    let current_id: i64 = conn.query_row(
        "SELECT id FROM players
         WHERE game_id = ?1 AND current_turn = 1",
        (game_id,),
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE players SET current_turn = 0 WHERE id = ?1",
        (current_id,),
    )?;

    let next_id: i64 = conn.query_row(
        "SELECT id FROM players
         WHERE game_id = ?1 AND turn_order >
            (SELECT turn_order FROM players WHERE id = ?2)
         ORDER BY turn_order ASC
         LIMIT 1",
        (game_id, current_id),
        |row| row.get(0),
    ).or_else(|_| {
        conn.query_row(
            "SELECT id FROM players
             WHERE game_id = ?1
             ORDER BY turn_order ASC
             LIMIT 1",
            (game_id,),
            |row| row.get(0),
        )
    })?;

    conn.execute(
        "UPDATE players SET current_turn = 1 WHERE id = ?1",
        (next_id,),
    )?;

    Ok(())
}

// =========================
// 🏁 게임 종료 체크
// =========================
fn check_game_end(conn: &Connection, player_id: i64) -> Result<bool> {
    let lap: i32 = conn.query_row(
        "SELECT lap FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    Ok(lap >= 3)
}

// =========================
// 🎮 한 턴 실행
// =========================
pub fn play_turn(conn: &Connection, game_id: i32) -> Result<()> {
    let (player_id, _, _) = get_current_player(conn, game_id)?;

    let dice = roll_dice();
    println!("🎲 player {} rolled {}", player_id, dice);

    move_player(conn, player_id, dice)?;
    handle_tile(conn, player_id)?;

    if check_game_end(conn, player_id)? {
        println!("🏁 player {} wins!", player_id);
        return Ok(());
    }

    next_turn(conn, game_id)?;

    Ok(())
}