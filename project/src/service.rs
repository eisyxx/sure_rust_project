use rand::Rng;
use rusqlite::{Connection, Result};


// 🎲 주사위
fn roll_dice() -> u8 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=6)
}


// 🎯 현재 턴 플레이어 가져오기
fn get_current_player(conn: &Connection, game_id: i32) -> Result<(i64, String, i32, i32)> {
    let mut stmt = conn.prepare(
        "SELECT id, name, position, lap FROM players
         WHERE game_id = ?1 AND current_turn = 1 AND is_bankrupt = 0"
    )?;

    let player = stmt.query_row((game_id,), |row| {
        Ok((
            row.get(0)?, // id
            row.get(1)?, // name
            row.get(2)?, // position
            row.get(3)?, // lap
        ))
    })?;

    Ok(player)
}


// 🧱 보드 크기
fn get_board_size(conn: &Connection) -> Result<i32> {
    conn.query_row(
        "SELECT COUNT(*) FROM tiles",
        [],
        |row| row.get(0),
    )
}


// 🚶 플레이어 이동
fn move_player(conn: &Connection, player_id: i64, dice: u8) -> Result<(i32, bool)> {
    let (position, lap): (i32, i32) = conn.query_row(
        "SELECT position, lap FROM players WHERE id = ?1",
        (player_id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let board_size = get_board_size(conn)?;
    let new_position = position + dice as i32;

    let passed_start = new_position >= board_size;
    let new_lap = if passed_start { lap + 1 } else { lap };

    let final_position = new_position % board_size;

    conn.execute(
        "UPDATE players SET position = ?1, lap = ?2 WHERE id = ?3",
        (final_position, new_lap, player_id),
    )?;

    Ok((final_position, passed_start))
}


// 🔄 턴 넘기기
fn next_turn(conn: &Connection, game_id: i32) -> Result<()> {
    let current_id_opt: Option<i64> = conn.query_row(
        "SELECT id FROM players
         WHERE game_id = ?1 AND current_turn = 1",
        (game_id,),
        |row| row.get(0),
    )?;

    let current_id = match current_id_opt {
        Some(id) => id,
        None => {
            // current_turn 플레이어 없으면 첫 번째 플레이어로 시작
            let first_id: i64 = conn.query_row(
                "SELECT id FROM players
                 WHERE game_id = ?1
                 ORDER BY turn_order ASC
                 LIMIT 1",
                (game_id,),
                |row| row.get(0),
            )?;
            // 바로 current_turn 세팅
            conn.execute(
                "UPDATE players SET current_turn = 1 WHERE id = ?1",
                (first_id,),
            )?;
            return Ok(());
        }
    };

    conn.execute(
        "UPDATE players SET current_turn = 0 WHERE id = ?1",
        (current_id,),
    )?;

    let next_id: i64 = match conn.query_row(
        "SELECT id FROM players
        WHERE game_id = ?1 AND turn_order > 
        (SELECT turn_order FROM players WHERE id = ?2)
        ORDER BY turn_order ASC
        LIMIT 1",
        (game_id, current_id),
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            // 마지막 플레이어였으면 첫 번째 플레이어로 돌아가기
            conn.query_row(
                "SELECT id FROM players
                WHERE game_id = ?1
                ORDER BY turn_order ASC
                LIMIT 1",
                (game_id,),
                |row| row.get(0),
            )?
        }
    };

    conn.execute(
        "UPDATE players SET current_turn = 1 WHERE id = ?1",
        (next_id,),
    )?;

    Ok(())
}


// 💰 월급
fn give_salary(conn: &Connection, player_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = money + 20 WHERE id = ?1",
        (player_id,),
    )?;

    let name: String = conn.query_row(
        "SELECT name FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    println!("💰 {} got +20 salary!", name);

    Ok(())
}


// 🏁 게임 종료 체크
fn check_game_end(conn: &Connection, player_id: i64) -> Result<bool> {
    let lap: i32 = conn.query_row(
        "SELECT lap FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    Ok(lap >= 3)
}


// 🎮 한 턴 실행
pub fn play_turn(conn: &Connection, game_id: i32) -> Result<(bool)> {
    let (player_id, player_name, old_position, _) = get_current_player(conn, game_id)?;

    let dice = roll_dice();
    let (new_position, passed_start) = move_player(conn, player_id, dice)?;

    let new_position: i32 = conn.query_row(
        "SELECT position FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    println!("🎲 {} rolled {} ({} -> {})", player_name, dice, old_position, new_position);

    // 월급 처리
    if passed_start {
        give_salary(conn, player_id)?;
    }

    if check_game_end(conn, player_id)? {
        println!("🏁 {} wins!", player_name);
        return Ok(true);
    }

    next_turn(conn, game_id)?;

    Ok(false)
}