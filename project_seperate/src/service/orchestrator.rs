use rusqlite::Connection;

use crate::repository::{
    player_repo::{PlayerRow, PlayerState, get_all_players, get_player_states},
    property_repo::{TileOwnerRecord, get_owned_tiles},
    transcaction_repo::{TransactionRecord, get_transactions_by_player},
};

use crate::service::traits::TurnServiceDeps;
use crate::service::traits::TurnRepo;

use crate::service::buy_property_service::{is_purchasable_tile, decide_buy_property, BuyResult};
use crate::service::game_end_service::{evaluate_and_apply_game_end, Player as GamePlayer};
use crate::service::turn_execute_service::{apply_turn_result, pre_apply_move_salary, apply_purchase};
use crate::service::turn_service::{
    build_landing_context, build_turn_result, roll_and_move, get_active_game_players,
    resolve_current_player_id, TurnAction, TurnServiceDepsImpl,roll_and_move_with_deps, TurnResult,
};
use crate::repository::init::init_db;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  세션 및 대기 상태 구조체
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
    pub final_rankings: Option<Vec<(i32, i32)>>,
    pub players: Vec<GamePlayer>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  서비스 반환용 도메인 결과 타입
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct StateResult {
    pub players: Vec<PlayerState>,
    pub tile_owners: Vec<TileOwnerRecord>,
    pub current_player_id: Option<i32>,
    pub game_finished: bool,
    pub winner_id: Option<i32>,
}

pub struct TurnOutcome {
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
    pub players: Vec<PlayerState>,
    pub tile_owners: Vec<TileOwnerRecord>,
    pub current_player_id: Option<i32>,
    pub game_finished: bool,
    pub winner_id: Option<i32>,
}

pub struct ResultPlayer {
    pub id: i32,
    pub money: i32,
    pub is_bankrupt: bool,
    pub rank: Option<usize>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  내부 유틸리티
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn map_action(action: &TurnAction) -> (&'static str, i32, Option<i32>) {
    match action {
        TurnAction::None => ("none", 0, None),
        TurnAction::Purchase { price } => ("purchase", *price, None),
        TurnAction::PayToll { owner_id, amount } => ("pay_toll", *amount, Some(*owner_id)),
        TurnAction::Bankrupt { owner_id, paid } => ("bankrupt", *paid, Some(*owner_id)),
        TurnAction::EventWelfareFund { amount } => ("welfare_fund", *amount, None),
        TurnAction::EventWelfareFundBankrupt { paid } => ("welfare_fund_bankrupt", *paid, None),
        TurnAction::EventFundReceive { amount } => ("fund_receive", *amount, None),
        TurnAction::FundReceiveEmpty => ("fund_receive_empty", 0, None),
        TurnAction::EstateTax { amount } => ("estate_tax", *amount, None),
        TurnAction::EstateTaxBankrupt { paid } => ("estate_tax_bankrupt", *paid, None),
        TurnAction::EstateTaxSkipped => ("estate_tax_skipped", 0, None),
    }
}

fn to_game_player(row: &PlayerRow) -> GamePlayer {
    GamePlayer {
        id: row.id,
        position: row.position,
        money: row.money,
        lap: row.lap,
        is_bankrupt: row.is_bankrupt,
    }
}

fn advance_turn(
    conn: &Connection,
    session: &mut SessionState,
    bankrupt_occurred: bool,
) -> rusqlite::Result<()> {
    let result = evaluate_and_apply_game_end(conn)?;
    session.game_finished = result.game_finished;

    if result.game_finished {
        session.winner_id = result.winner_id;
        session.final_rankings = result.rankings;
    } else {
        if !bankrupt_occurred {
            session.current_turn_index += 1;
        }
        session.winner_id = None;
        session.final_rankings = None;
    }

    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  trait
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct TurnRepoImpl;

impl TurnRepo for TurnRepoImpl {
    fn get_active_game_players(&self, conn: &Connection) -> rusqlite::Result<Vec<GamePlayer>> {
        get_active_game_players(conn)
    }
    fn get_active_players(&self, conn: &Connection) -> rusqlite::Result<Vec<GamePlayer>> {
        get_active_game_players(conn)
    }
    fn get_player_states(&self, conn: &Connection) -> rusqlite::Result<Vec<PlayerState>> {
        get_player_states(conn)
    }
    fn get_owned_tiles(&self, conn: &Connection) -> rusqlite::Result<Vec<TileOwnerRecord>> {
        get_owned_tiles(conn)
    }
    fn resolve_current_player_id(&self, conn: &Connection, idx: usize) -> rusqlite::Result<Option<i32>> {
        resolve_current_player_id(conn, idx)
    }
    fn apply_turn_result(&self, conn: &Connection, player_id: i32, result: &TurnResult) -> rusqlite::Result<()> {
        apply_turn_result(conn, player_id, result)
    }
    fn pre_apply_move_salary(&self, conn: &Connection, player_id: i32, pos: i32, lap: i32, salary: i32) -> rusqlite::Result<()> {
        pre_apply_move_salary(conn, player_id, pos, lap, salary)
    }
    fn apply_purchase(&self, conn: &Connection, player_id: i32, pos: i32, price: i32) -> rusqlite::Result<()> {
        apply_purchase(conn, player_id, pos, price)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  공개 서비스 함수
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 세션 초기화 (index 페이지 진입 시)
pub fn init_session(conn: &Connection) -> rusqlite::Result<SessionState> {
    init_db::init_db(conn)?; // [DB 초기화] repository 직접 호출

    let db_players = get_all_players(conn)?; // [DB 읽기] repository 직접 호출
    let game_players: Vec<GamePlayer> = db_players.iter().map(to_game_player).collect();

    Ok(SessionState {
        current_turn_index: 0,
        game_finished: false,
        winner_id: None,
        pending: None,
        final_rankings: None,
        players: game_players,
    })
}

/// 현재 게임 상태 조회
pub fn get_state(conn: &Connection, session: &SessionState) -> rusqlite::Result<StateResult> {
    let players = get_player_states(conn)?; // [DB 읽기] repository 직접 호출
    let current_player_id = resolve_current_player_id(conn, session.current_turn_index)?;
    let tile_owners = get_owned_tiles(conn)?; // [DB 읽기] repository 직접 호출

    Ok(StateResult {
        players,
        tile_owners,
        current_player_id,
        game_finished: session.game_finished,
        winner_id: session.winner_id,
    })
}

/// 특정 플레이어의 거래 내역 조회
pub fn get_transactions(conn: &Connection, player_id: i32) -> rusqlite::Result<Vec<TransactionRecord>> {
    get_transactions_by_player(conn, player_id) // [DB 읽기] repository 직접 호출
}

/// 테스트 용
pub fn process_turn(
    conn: &Connection,
    session: &mut SessionState,
) -> rusqlite::Result<TurnOutcome> {
    let repo = TurnRepoImpl;
    let deps = TurnServiceDepsImpl;
    process_turn_with_repo(&repo, &deps, conn, session)
}

/// 한 턴 진행 (기본)
pub fn process_turn_with_repo<R: TurnRepo, D: TurnServiceDeps>(repo: &R, deps: &D, conn: &Connection, session: &mut SessionState) -> rusqlite::Result<TurnOutcome> {
    let players = repo.get_active_game_players(conn)?;

    if players.is_empty() {
        advance_turn(conn, session, false)?;

        return Ok(TurnOutcome {
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
            game_finished: session.game_finished,
            winner_id: session.winner_id,
        });
    }

    if session.current_turn_index >= players.len() {
        session.current_turn_index = 0;
    }

    let current_player = &players[session.current_turn_index];

    let move_step = roll_and_move_with_deps(deps, current_player.position, current_player.lap, 24);

    let landing = build_landing_context(
        conn,
        move_step.new_position,
        current_player.money,
        move_step.salary,
    );
    let tile_price = landing.tile_price;
    let tile_toll = landing.tile_toll;
    let tile_owner = landing.tile_owner;
    let tile_type = landing.tile_type;
    let money_after_salary = landing.money_after_salary;

    // 구매 가능 타일 → 구매 여부를 클라이언트에 질의
    if is_purchasable_tile(tile_owner, &tile_type, tile_price) {
        repo.pre_apply_move_salary(conn, current_player.id, move_step.new_position, move_step.new_lap, move_step.salary)?;

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

        let players_after = repo.get_player_states(conn)?; // [DB 읽기] repository 직접 호출
        let tile_owners = repo.get_owned_tiles(conn)?; // [DB 읽기] repository 직접 호출
        let cpi = repo.resolve_current_player_id(conn, session.current_turn_index)?;

        return Ok(TurnOutcome {
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
            players: players_after,
            tile_owners,
            current_player_id: cpi,
            game_finished: false,
            winner_id: None,
        });
    }

    // 구매 불가 타일 → 턴 즉시 완료
    let old_position = current_player.position;
    let old_lap = current_player.lap;
    let player_id = current_player.id;

    let turn_result = build_turn_result(
        conn,
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

    let bankrupt_occurred = turn_result.action.is_bankrupt();
    advance_turn(conn, session, bankrupt_occurred)?;

    let players_after = get_player_states(conn)?; // [DB 읽기] repository 직접 호출
    let tile_owners = get_owned_tiles(conn)?; // [DB 읽기] repository 직접 호출
    let current_player_id = resolve_current_player_id(conn, session.current_turn_index)?;

    let (action_type, action_amount, owner_id) = map_action(&turn_result.action);

    Ok(TurnOutcome {
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
        players: players_after,
        tile_owners,
        current_player_id,
        game_finished: session.game_finished,
        winner_id: session.winner_id,
    })
}

/// 구매 결정 처리
pub fn process_decide(
    conn: &Connection,
    session: &mut SessionState,
    will_buy: bool,
) -> rusqlite::Result<TurnOutcome> {
    let pending = match session.pending.take() {
        Some(p) => p,
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    };

    let buy_result = decide_buy_property(
        pending.player_id,
        pending.money_after_salary,
        pending.tile_price,
        0,
        None,
        will_buy,
        "property".to_string(),
    );

    let (action_type, action_amount) = match &buy_result {
        BuyResult::Purchase { price } => {
            apply_purchase(conn, pending.player_id, pending.new_position, *price)?;
            ("purchase", *price)
        }
        _ => ("skip", 0),
    };

    advance_turn(conn, session, false)?;

    let players_after = get_player_states(conn)?; // [DB 읽기] repository 직접 호출
    let tile_owners = get_owned_tiles(conn)?; // [DB 읽기] repository 직접 호출
    let current_player_id = resolve_current_player_id(conn, session.current_turn_index)?;

    Ok(TurnOutcome {
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
        players: players_after,
        tile_owners,
        current_player_id,
        game_finished: session.game_finished,
        winner_id: session.winner_id,
    })
}

/// 게임 결과 조회
pub fn get_result(conn: &Connection, session: &SessionState) -> Vec<ResultPlayer> {
    let all_players = match get_all_players(conn) { // [DB 읽기] repository 직접 호출
        Ok(players) => players,
        Err(_) => return vec![],
    };

    if let Some(rankings) = &session.final_rankings {
        rankings
            .iter()
            .enumerate()
            .map(|(i, (player_id, money))| {
                let player_opt = all_players.iter().find(|p| p.id == *player_id);
                let is_bankrupt = player_opt.map(|p| p.is_bankrupt).unwrap_or(true);

                ResultPlayer {
                    id: *player_id,
                    money: *money,
                    is_bankrupt,
                    rank: if is_bankrupt { None } else { Some(i + 1) },
                }
            })
            .collect()
    } else {
        all_players
            .iter()
            .map(|p| ResultPlayer {
                id: p.id,
                money: p.money,
                is_bankrupt: p.is_bankrupt,
                rank: None,
            })
            .collect()
    }
}

/// 게임 리셋
pub fn reset_game(conn: &Connection, session: &mut SessionState) -> rusqlite::Result<()> {
    init_db::init_db(conn)?; // [DB 초기화] repository 직접 호출

    session.current_turn_index = 0;
    session.game_finished = false;
    session.winner_id = None;
    session.pending = None;
    session.final_rankings = None;

    Ok(())
}
