use rusqlite::Connection;
use serde::Serialize;

use crate::repository::{
    player_repo::{get_all_players, get_player_states, update_money, update_position_and_lap, PlayerState},
    property_repo::{get_owned_tiles, get_owner, set_owner},
    tile_repo::get_tile_info,
    transcaction_repo::{get_transactions_by_player, record_transaction},
};
use crate::service::game_end_service::{check_game_end, Player as GamePlayer};
use crate::service::turn_execute_service::apply_turn_result;
use crate::service::turn_service::{build_turn_result, roll_and_move, TurnAction};

/// 구매 결정 대기 중인 턴 상태
pub struct PendingTurn {
    pub player_id: i32,
    pub dice: i32,
    pub old_position: i32,
    pub new_position: i32,
    pub old_lap: i32,
    pub new_lap: i32,
    pub salary: i32,
    pub tile_price: i32,
    pub money_after_salary: i32,
}

pub struct SessionState {
    pub current_turn_index: usize,
    pub game_finished: bool,
    pub winner_id: Option<i32>,
    pub pending: Option<PendingTurn>,
}

#[derive(Serialize)]
pub struct ApiPlayer {
    pub id: i32,
    pub name: String,
    pub position: i32,
    pub money: i32,
    pub lap: i32,
    pub turn_order: i32,
    pub is_bankrupt: bool,
}

#[derive(Serialize)]
pub struct ApiTransaction {
    pub id: i32,
    pub tx_type: String,
    pub amount: i32,
    pub target: String,
    pub balance_before: i32,
    pub balance_after: i32,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ApiTileOwner {
    pub tile_id: i32,
    pub owner_id: i32,
}

#[derive(Serialize)]
pub struct ApiStateResponse {
    pub players: Vec<ApiPlayer>,
    pub tile_owners: Vec<ApiTileOwner>,
    pub current_player_id: Option<i32>,
    pub game_finished: bool,
    pub winner_id: Option<i32>,
}

#[derive(Serialize)]
pub struct ApiTurnResponse {
    pub player_id: i32,
    pub dice: i32,
    pub old_position: i32,
    pub new_position: i32,
    pub old_lap: i32,
    pub new_lap: i32,
    pub salary: i32,
    pub action_type: &'static str,
    pub action_amount: i32,
    pub owner_id: Option<i32>,
    pub players: Vec<ApiPlayer>,
    pub tile_owners: Vec<ApiTileOwner>,
    pub current_player_id: Option<i32>,
    pub game_finished: bool,
    pub winner_id: Option<i32>,
}

fn active_players(players: &[PlayerState]) -> Vec<&PlayerState> {
    players.iter().filter(|player| !player.is_bankrupt).collect()
}

fn current_player_id(players: &[PlayerState], current_turn_index: usize) -> Option<i32> {
    let active = active_players(players);

    if active.is_empty() {
        return None;
    }

    let normalized_index = current_turn_index % active.len();
    Some(active[normalized_index].id)
}

fn map_players(players: Vec<PlayerState>) -> Vec<ApiPlayer> {
    players
        .into_iter()
        .map(|player| ApiPlayer {
            id: player.id,
            name: player.name,
            position: player.position,
            money: player.money,
            lap: player.lap,
            turn_order: player.turn_order,
            is_bankrupt: player.is_bankrupt,
        })
        .collect()
}

fn map_tile_owners(conn: &Connection) -> rusqlite::Result<Vec<ApiTileOwner>> {
    let records = get_owned_tiles(conn)?;

    Ok(records
        .into_iter()
        .map(|record| ApiTileOwner {
            tile_id: record.tile_id,
            owner_id: record.owner_id,
        })
        .collect())
}

pub fn get_state(conn: &Connection, session: &SessionState) -> rusqlite::Result<ApiStateResponse> {
    let players = get_player_states(conn)?;
    let current_player_id = current_player_id(&players, session.current_turn_index);
    let tile_owners = map_tile_owners(conn)?;

    Ok(ApiStateResponse {
        players: map_players(players),
        tile_owners,
        current_player_id,
        game_finished: session.game_finished,
        winner_id: session.winner_id,
    })
}

pub fn get_transactions(conn: &Connection, player_id: i32) -> rusqlite::Result<Vec<ApiTransaction>> {
    let transactions = get_transactions_by_player(conn, player_id)?;

    Ok(transactions
        .into_iter()
        .map(|tx| ApiTransaction {
            id: tx.id,
            tx_type: tx.tx_type,
            amount: tx.amount,
            target: tx.target,
            balance_before: tx.balance_before,
            balance_after: tx.balance_after,
            created_at: tx.created_at,
        })
        .collect())
}

pub fn handle_turn(conn: &Connection, session: &mut SessionState) -> rusqlite::Result<ApiTurnResponse> {
    let players = get_all_players(conn)?
        .into_iter()
        .filter(|player| !player.is_bankrupt)
        .collect::<Vec<_>>();

    if players.is_empty() {
        session.game_finished = true;

        return Ok(ApiTurnResponse {
            player_id: 0,
            dice: 0,
            old_position: 0,
            new_position: 0,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "none",
            action_amount: 0,
            owner_id: None,
            players: vec![],
            tile_owners: vec![],
            current_player_id: None,
            game_finished: true,
            winner_id: None,
        });
    }

    if session.current_turn_index >= players.len() {
        session.current_turn_index = 0;
    }

    let current_player = &players[session.current_turn_index];
    let move_step = roll_and_move(current_player.position, current_player.lap, 24);

    let (tile_price, tile_toll, _, tile_type) =
        get_tile_info(conn, move_step.new_position).unwrap_or((0, 0, None, String::from("unknown")));
    let tile_owner = get_owner(conn, move_step.new_position).unwrap_or(None);
    let money_after_salary = current_player.money + move_step.salary;

    // 소유자 없는 구매 가능 타일 → 플레이어에게 구매 여부 묻기
    if tile_owner.is_none() && tile_type != "event" && tile_price > 0 {
        // 이동 + 월급만 선반영
        update_position_and_lap(conn, current_player.id, move_step.new_position, move_step.new_lap)?;
        if move_step.salary > 0 {
            update_money(conn, current_player.id, move_step.salary)?;
            record_transaction(conn, current_player.id, "deposit", move_step.salary, "salary")?;
        }

        session.pending = Some(PendingTurn {
            player_id: current_player.id,
            dice: move_step.dice,
            old_position: current_player.position,
            new_position: move_step.new_position,
            old_lap: current_player.lap,
            new_lap: move_step.new_lap,
            salary: move_step.salary,
            tile_price,
            money_after_salary,
        });

        let players_after = get_player_states(conn)?;
        let tile_owners = map_tile_owners(conn)?;
        let cpi = current_player_id(&players_after, session.current_turn_index);

        return Ok(ApiTurnResponse {
            player_id: current_player.id,
            dice: move_step.dice,
            old_position: current_player.position,
            new_position: move_step.new_position,
            old_lap: current_player.lap,
            new_lap: move_step.new_lap,
            salary: move_step.salary,
            action_type: "can_buy",
            action_amount: tile_price,
            owner_id: None,
            players: map_players(players_after),
            tile_owners,
            current_player_id: cpi,
            game_finished: false,
            winner_id: None,
        });
    }

    // 그 외 (통행료 / 이벤트 / 빈 타일) → 즉시 실행
    let old_position = current_player.position;
    let old_lap = current_player.lap;
    let player_id = current_player.id;

    let turn_result = build_turn_result(
        move_step,
        player_id,
        money_after_salary,
        tile_price,
        tile_toll,
        tile_owner,
        false,
        &tile_type,
    );
    apply_turn_result(conn, player_id, &turn_result)?;

    advance_turn(conn, session, player_id)?;

    let players_after = get_player_states(conn)?;
    let tile_owners = map_tile_owners(conn)?;
    let current_player_id = current_player_id(&players_after, session.current_turn_index);

    let (action_type, action_amount, owner_id) = match &turn_result.action {
        TurnAction::None => ("none", 0, None),
        TurnAction::Purchase { price } => ("purchase", *price, None),
        TurnAction::PayToll { owner_id, amount } => ("pay_toll", *amount, Some(*owner_id)),
        TurnAction::Bankrupt { owner_id, paid } => ("bankrupt", *paid, Some(*owner_id)),
    };

    Ok(ApiTurnResponse {
        player_id,
        dice: turn_result.dice,
        old_position,
        new_position: turn_result.new_position,
        old_lap,
        new_lap: turn_result.new_lap,
        salary: turn_result.salary,
        action_type,
        action_amount,
        owner_id,
        players: map_players(players_after),
        tile_owners,
        current_player_id,
        game_finished: session.game_finished,
        winner_id: session.winner_id,
    })
}

/// 구매 결정을 처리하고 턴을 완료한다
pub fn handle_decide(
    conn: &Connection,
    session: &mut SessionState,
    will_buy: bool,
) -> rusqlite::Result<ApiTurnResponse> {
    let pending = match session.pending.take() {
        Some(p) => p,
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    };

    if will_buy && pending.money_after_salary >= pending.tile_price {
        update_money(conn, pending.player_id, -pending.tile_price)?;
        record_transaction(conn, pending.player_id, "withdraw", pending.tile_price, "tile_purchase")?;
        set_owner(conn, pending.new_position, pending.player_id, pending.tile_price)?;
    }

    advance_turn(conn, session, pending.player_id)?;

    let players_after = get_player_states(conn)?;
    let tile_owners = map_tile_owners(conn)?;
    let current_player_id = current_player_id(&players_after, session.current_turn_index);

    let (action_type, action_amount) = if will_buy && pending.money_after_salary >= pending.tile_price {
        ("purchase", pending.tile_price)
    } else {
        ("skip", 0)
    };

    Ok(ApiTurnResponse {
        player_id: pending.player_id,
        dice: pending.dice,
        old_position: pending.old_position,
        new_position: pending.new_position,
        old_lap: pending.old_lap,
        new_lap: pending.new_lap,
        salary: pending.salary,
        action_type,
        action_amount,
        owner_id: None,
        players: map_players(players_after),
        tile_owners,
        current_player_id,
        game_finished: session.game_finished,
        winner_id: session.winner_id,
    })
}

fn advance_turn(
    conn: &Connection,
    session: &mut SessionState,
    _player_id: i32,
) -> rusqlite::Result<()> {
    let all_players = get_all_players(conn)?
        .into_iter()
        .map(|player| GamePlayer {
            id: player.id,
            position: player.position,
            money: player.money,
            lap: player.lap,
            is_bankrupt: player.is_bankrupt,
        })
        .collect::<Vec<_>>();

    let game_result = check_game_end(all_players);
    session.game_finished = game_result.is_finished;
    session.winner_id = game_result.winner_id;

    if !session.game_finished {
        session.current_turn_index += 1;
    }

    Ok(())
}