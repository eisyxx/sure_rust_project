use rand::Rng;
use rusqlite::{Connection, Result};
use crate::dto::{TurnResponse, GameState, PlayerState, TransactionHistoryResponse, TransactionItem};
use crate::init_db::create_db::init_db;
use crate::init_db::init_player::create_player;
use crate::init_db::init_tiles::init_tiles;

pub fn reset_game(conn: &Connection, game_id: i32) -> Result<()> {
    init_db(conn)?;

    conn.execute("DELETE FROM transactions", [])?;
    conn.execute("DELETE FROM properties", [])?;
    conn.execute("DELETE FROM event_tiles", [])?;
    conn.execute("DELETE FROM fund", [])?;
    conn.execute("DELETE FROM players", [])?;
    conn.execute("DELETE FROM games", [])?;
    conn.execute("DELETE FROM tiles", [])?;

    conn.execute(
        "INSERT INTO games(current_turn, status)
         VALUES (?1, 'playing')",
        (1,),
    )?;

    init_tiles(conn)?;

    create_player(conn, game_id, "Player1", 1)?;
    create_player(conn, game_id, "Player2", 2)?;
    create_player(conn, game_id, "Player3", 3)?;
    create_player(conn, game_id, "Player4", 4)?;

    conn.execute("UPDATE players SET current_turn = 0", [])?;
    conn.execute(
        "UPDATE players
         SET current_turn = 1
         WHERE game_id = ?1 AND turn_order = 1",
        (game_id,),
    )?;

    Ok(())
}

// 게임 상태 조회
pub fn get_game_state(conn: &Connection, game_id: i32) -> Result<GameState> {
    // 모든 플레이어 정보 조회
    let mut stmt = conn.prepare(
        "SELECT id, name, position, lap, money, turn_order FROM players
         WHERE game_id = ?1
         ORDER BY turn_order ASC"
    )?;

    let players = stmt.query_map((game_id,), |row| {
        Ok(PlayerState {
            id: row.get(0)?,
            name: row.get(1)?,
            position: row.get(2)?,
            lap: row.get(3)?,
            money: row.get(4)?,
            turn_order: row.get(5)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    // 현재 턴 플레이어 ID
    let current_player_id: i64 = conn.query_row(
        "SELECT id FROM players WHERE game_id = ?1 AND current_turn = 1",
        (game_id,),
        |row| row.get(0),
    )?;

    Ok(GameState {
        players,
        current_player_id,
    })
}

// API용
pub fn play_turn_api(conn: &Connection, game_id: i32) -> Result<TurnResponse> {
    let (player_id, player_name, old_position, _) = get_current_player(conn, game_id)?;

    let dice = roll_dice();
    let (new_position, passed_start) = move_player(conn, player_id, dice)?;

    if passed_start {
        give_salary(conn, player_id)?;
    }

    let game_end = check_game_end(conn, player_id)?;

    Ok(TurnResponse {
        player_id,
        dice,
        old_position,
        new_position,
        passed_start,
        game_end,
    })
}


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
pub fn next_turn(conn: &Connection, game_id: i32) -> Result<()> {
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
    let salary_amount = 20;

    conn.execute(
        "UPDATE players SET money = money + ?1 WHERE id = ?2",
        (salary_amount, player_id),
    )?;

    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, created_at)
         VALUES (?1, 'deposit', ?2, '월급', datetime('now', '+9 hours'))",
        (player_id, salary_amount),
    )?;

    let name: String = conn.query_row(
        "SELECT name FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    println!("💰 {} got +{} salary!", name, salary_amount);

    Ok(())
}


// 🏁 게임 종료 체크
fn check_game_end(conn: &Connection, player_id: i64) -> Result<bool> {
    let lap: i32 = conn.query_row(
        "SELECT lap FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    // 3바퀴 완주하면 게임 종료
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

pub fn get_current_player_transactions(
    conn: &Connection,
    game_id: i32,
) -> Result<TransactionHistoryResponse> {
    let (player_id, player_name, _, _) = get_current_player(conn, game_id)?;

    let mut stmt = conn.prepare(
        "SELECT id, type, amount, target, created_at
         FROM transactions
         WHERE player_id = ?1
         ORDER BY id DESC",
    )?;

    let transactions = stmt
        .query_map((player_id,), |row| {
            Ok(TransactionItem {
                id: row.get(0)?,
                tx_type: row.get(1)?,
                amount: row.get(2)?,
                target: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TransactionHistoryResponse {
        player_id,
        player_name,
        transactions,
    })
}

// 토지 소유자 확인
#[derive(Debug)]
pub enum PropertyResult {
    TollPaid,   // 통행료 지불 발생
    CanBuy,     // 구매 가능 (프론트에서 버튼 띄움)
    Nothing,    // 아무 일도 없음 (내 땅 등)
}
pub fn handle_property(conn: &Connection, player_id: i64) -> Result<PropertyResult> {
    let position: i32 = conn.query_row(
        "SELECT position FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    let owner: Option<i64> = conn.query_row(
        "SELECT owner_id FROM properties WHERE tile_id = ?1",
        (position,),
        |row| row.get(0),
    ).ok();

    match owner {
        // 🔴 다른 사람 땅 → 통행료 즉시 처리
        Some(owner_id) if owner_id != player_id => {
            let toll: i32 = conn.query_row(
                "SELECT toll FROM tiles WHERE id = ?1",
                (position,),
                |row| row.get(0),
            )?;

            handle_toll(conn, player_id, owner_id, toll)?;
            return Ok(PropertyResult::TollPaid);
        }

        // 🟢 빈 땅 → 구매 가능
        None => {
            return Ok(PropertyResult::CanBuy);
        }

        // 🟡 내 땅
        _ => {
            return Ok(PropertyResult::Nothing);
        }
    }
}

// 통행료 처리
fn handle_toll(conn: &Connection, player_id: i64, owner_id: i64, toll: i32) -> Result<()> {
    let money: i32 = conn.query_row(
        "SELECT money FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    let pay_amount = if money >= toll { toll } else { money };

    // 돈 이동
    conn.execute(
        "UPDATE players SET money = money - ?1 WHERE id = ?2",
        (pay_amount, player_id),
    )?;

    conn.execute(
        "UPDATE players SET money = money + ?1 WHERE id = ?2",
        (pay_amount, owner_id),
    )?;

    // 거래 기록
    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, created_at)
         VALUES (?1, 'withdraw', ?2, '통행료', datetime('now', '+9 hours'))",
        (player_id, pay_amount),
    )?;

    // 파산 처리
    if money < toll {
        conn.execute(
            "UPDATE players SET is_bankrupt = 1 WHERE id = ?1",
            (player_id,),
        )?;
        conn.execute(
            "UPDATE players SET money = money - ?1 WHERE id = ?2",
            (money, player_id),
        )?;

        conn.execute(
            "UPDATE players SET money = money + ?1 WHERE id = ?2",
            (money, owner_id),
        )?;
        conn.execute(
            "INSERT INTO transactions (player_id, type, amount, target, created_at)
            VALUES (?1, 'withdraw', ?2, '통행료(파산)', datetime('now', '+9 hours'))",
            (player_id, money),
        )?;
        println!("💥 Player {} bankrupt!", player_id);
    }

    Ok(())
}

// 토지 구매
pub fn buy_property(conn: &Connection, player_id: i64, tile_id: i32, price: i32) -> Result<()> {
    let money: i32 = conn.query_row(
        "SELECT money FROM players WHERE id = ?1",
        (player_id,),
        |row| row.get(0),
    )?;

    if money < price {
    return Err(rusqlite::Error::ExecuteReturnedResults);
    }

    // 돈 차감
    conn.execute(
        "UPDATE players SET money = money - ?1 WHERE id = ?2",
        (price, player_id),
    )?;

    // property 등록
    conn.execute(
        "UPDATE properties (tile_id, owner_id, price)
         VALUES (?1, ?2, ?3)",
        (tile_id, player_id, price),
    )?;

    // 거래 기록
    conn.execute(
        "INSERT INTO transactions (player_id, type, amount, target, created_at)
         VALUES (?1, 'withdraw', ?2, '토지 구매', datetime('now', '+9 hours'))",
        (player_id, price),
    )?;

    println!("🏠 Player {} bought tile {}", player_id, tile_id);

    Ok(())
}

// 상금 지급 + 승자 발표
pub fn distribute_rewards(conn: &Connection) -> Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT id, money, lap, position
         FROM players
         WHERE is_bankrupt = 0"
    )?;

    let mut players: Vec<(i64, i32, i32, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?, // id
                row.get(1)?, // money
                row.get(2)?, // lap
                row.get(3)?, // position
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // 정렬: lap DESC → position DESC
    players.sort_by(|a, b| {
        b.2.cmp(&a.2) // lap
            .then(b.3.cmp(&a.3)) // position
    });

    let rewards = vec![150, 120, 80];

    for (i, (player_id, _, _, _)) in players.iter().enumerate() {
        if i >= rewards.len() {
            break;
        }

        let reward = rewards[i];

        conn.execute(
            "UPDATE players SET money = money + ?1 WHERE id = ?2",
            (reward, player_id),
        )?;

        conn.execute(
            "INSERT INTO transactions (player_id, type, amount, target, created_at)
             VALUES (?1, 'deposit', ?2, '상금', datetime('now', '+9 hours'))",
            (player_id, reward),
        )?;

        println!("🏆 Player {} gets {}", player_id, reward);
    }

    // 최종 승자 (money 기준)
    let winner_id: i64 = conn.query_row(
        "SELECT id FROM players
         ORDER BY money DESC
         LIMIT 1",
        [],
        |row| row.get(0),
    )?;

    println!("🎉 Final Winner: Player {}", winner_id);

    Ok(winner_id)
}