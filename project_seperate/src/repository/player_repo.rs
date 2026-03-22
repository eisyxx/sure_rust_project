use rusqlite::{Connection, Result};

use crate::service::game_end_service::Player;

#[derive(Clone)]
pub struct PlayerState {
    pub id: i32,
    pub name: String,
    pub position: i32,
    pub money: i32,
    pub lap: i32,
    pub turn_order: i32,
    pub is_bankrupt: bool,
}

// 지정 플레이어의 잔액 불러오기
pub fn get_player_money(conn: &Connection, player_id: i32) -> Result<i32> {
    conn.query_row(
        "SELECT money FROM players WHERE id = ?1",
        [player_id],
        |row| row.get(0),
    )
}

// 지정 플레이어의 잔액 정보 업데이트
pub fn update_money(conn: &Connection, player_id: i32, delta: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = money + ?1 WHERE id = ?2",
        (delta, player_id),
    )?;
    Ok(())
}

// 지정 플레이어의 상태를 파산(is_bankrupt)으로 변경
pub fn bankrupt(conn: &Connection, player_id: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = 0, is_bankrupt = 1 WHERE id = ?1",
        [player_id],
    )?;
    Ok(())
}

// 현재 플레이 중인 모든 플레이어의 상태(id, 위치, 잔액, lap, 파산 여부)를 가져오기
pub fn get_all_players(conn: &Connection) -> Result<Vec<Player>> {
    let mut stmt = conn.prepare(
        "SELECT id, position, money, lap, is_bankrupt FROM players ORDER BY turn_order"
    )?;

    let players = stmt.query_map([], |row| {
        Ok(Player {
            id: row.get(0)?,
            position: row.get(1)?,
            money: row.get(2)?,
            lap: row.get(3)?,
            is_bankrupt: row.get::<_, i32>(4)? == 1,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(players)
}

// 플레이어의 상태 가져오기
pub fn get_player_states(conn: &Connection) -> Result<Vec<PlayerState>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, position, money, lap, turn_order, is_bankrupt
         FROM players
         ORDER BY turn_order"
    )?;

    let players = stmt.query_map([], |row| {
        Ok(PlayerState {
            id: row.get(0)?,
            name: row.get(1)?,
            position: row.get(2)?,
            money: row.get(3)?,
            lap: row.get(4)?,
            turn_order: row.get(5)?,
            is_bankrupt: row.get::<_, i32>(6)? == 1,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(players)
}

// 플레이어의 잔액을 지정된 값으로 업데이트 (이벤트, 월급)
pub fn update_player_money(conn: &Connection, player_id: i32, money: i32) -> Result<()> {
    conn.execute(
        "UPDATE players SET money = ?1 WHERE id = ?2",
        (money, player_id),
    )?;
    Ok(())
}

// 플레이어의 현재 위치(pos)와 진행한 바퀴 수(lap)를 업데이트
pub fn update_position_and_lap(
    conn: &Connection,
    player_id: i32,
    pos: i32,
    lap: i32,
) -> Result<()> {
    conn.execute(
        "UPDATE players SET position = ?1, lap = ?2 WHERE id = ?3",
        (pos, lap, player_id),
    )?;
    Ok(())
}

// 플레이어에게 상금 지급: 잔액 증가 후 거래 내역 기록
pub fn give_reward(conn: &Connection, player_id: i32, amount: i32) -> Result<()> {
    update_money(conn, player_id, amount)?;
    
    use crate::repository::transcaction_repo::record_transaction;
    record_transaction(conn, player_id, "deposit", amount, "game_reward")?;
    
    Ok(())
}