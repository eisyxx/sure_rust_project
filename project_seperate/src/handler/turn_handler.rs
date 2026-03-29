//! # 턴 핸들러 (Turn Handler)
//!
//! 보드게임의 턴 진행, 구매 결정, 게임 상태 조회 등
//! API 요청을 처리하는 핸들러 모듈.
//!
//! ## 주요 기능
//! - `get_state`: 현재 게임 상태 조회
//! - `get_transactions`: 특정 플레이어의 거래 내역 조회
//! - `handle_turn`: 한 턴 진행 (주사위 → 이동 → 액션 처리)
//! - `handle_decide`: 구매 가능 타일에 대한 구매/패스 결정 처리

use rusqlite::Connection;
use serde::Serialize;

// ── Repository 의존성 ──
use crate::repository::{
    player_repo::{get_all_players, get_player_states, PlayerState, PlayerRow},
    property_repo::{get_owned_tiles, get_owner},
    tile_repo::get_tile_info,
    transcaction_repo::get_transactions_by_player,
};

// ── Service 의존성 ──
use crate::service::buy_property_service::{is_purchasable_tile, decide_buy_property, BuyResult};
use crate::service::game_end_service::{check_game_end, apply_rewards, Player as GamePlayer};
use crate::service::turn_execute_service::{apply_turn_result, pre_apply_move_salary, apply_purchase};
use crate::service::turn_service::{build_turn_result, roll_and_move, TurnAction};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  내부 변환 헬퍼 함수
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// DB 조회용 `PlayerRow`를 게임 로직용 `GamePlayer`로 변환한다.
fn to_game_player(row: &PlayerRow) -> GamePlayer {
    GamePlayer {
        id: row.id,
        position: row.position,
        money: row.money,
        lap: row.lap,
        is_bankrupt: row.is_bankrupt,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  세션 및 대기 상태 구조체
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 구매 결정 대기 중인 턴 상태.
///
/// 플레이어가 구매 가능한 타일에 도착했을 때,
/// 구매 여부를 클라이언트로부터 받기 전까지 보관하는 중간 상태.
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

/// 게임 세션의 전체 상태를 관리하는 구조체.
///
/// 현재 턴 인덱스, 게임 종료 여부, 승자, 대기 중인 구매 결정 등
/// 한 게임 세션에 필요한 모든 상태를 보관한다.
pub struct SessionState {
    /// 활성 플레이어 목록 내에서의 현재 턴 인덱스
    pub current_turn_index: usize,
    /// 게임 종료 여부
    pub game_finished: bool,
    /// 게임 종료 시 승자의 플레이어 ID
    pub winner_id: Option<i32>,
    /// 구매 결정 대기 상태 (구매 가능 타일에 도착 시 설정)
    pub pending: Option<PendingTurn>,
    /// 게임 종료 시 최종 순위 (player_id, 보상금)
    pub final_rankings: Option<Vec<(i32, i32)>>,
    /// 현재 게임에 참여 중인 플레이어 목록
    pub players: Vec<GamePlayer>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  API 응답용 DTO (Data Transfer Object)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// API로 반환할 플레이어 정보
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

/// API로 반환할 거래 내역 정보
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

/// API로 반환할 타일(토지) 소유 정보
#[derive(Serialize)]
pub struct ApiTileOwner {
    pub tile_id: i32,
    pub owner_id: i32,
}

/// 게임 전체 상태 조회 API의 응답 구조체
#[derive(Serialize)]
pub struct ApiStateResponse {
    pub players: Vec<ApiPlayer>,
    pub tile_owners: Vec<ApiTileOwner>,
    pub current_player_id: Option<i32>,
    pub game_finished: bool,
    pub winner_id: Option<i32>,
}

/// 턴 진행 / 구매 결정 API의 응답 구조체.
/// 턴 결과(주사위, 이동, 액션)와 갱신된 게임 상태를 함께 반환한다.
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  내부 유틸리티 함수
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 파산하지 않은 활성 플레이어만 필터링하여 반환한다.
fn active_players(players: &[PlayerState]) -> Vec<&PlayerState> {
    players.iter().filter(|player| !player.is_bankrupt).collect()
}

/// 현재 턴 인덱스를 활성 플레이어 수로 나눈 나머지로
/// 실제 턴을 진행할 플레이어의 ID를 반환한다.
/// 활성 플레이어가 없으면 `None`을 반환한다.
fn current_player_id(players: &[PlayerState], current_turn_index: usize) -> Option<i32> {
    let active = active_players(players);

    if active.is_empty() {
        return None;
    }

    let normalized_index = current_turn_index % active.len();
    Some(active[normalized_index].id)
}

/// 내부 `PlayerState` 목록을 API 응답용 `ApiPlayer` 목록으로 변환한다.
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

/// DB에서 모든 타일의 소유 정보를 조회하여 API 응답 형태로 변환한다.
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  공개 핸들러 함수 (API 엔드포인트에서 호출)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 현재 게임 상태를 조회하여 `ApiStateResponse`로 반환한다.
///
/// 플레이어 목록, 타일 소유 현황, 현재 턴 플레이어, 게임 종료 여부를 포함한다.
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

/// 특정 플레이어의 전체 거래 내역을 조회하여 API 응답 형태로 반환한다.
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

/// 한 턴을 진행하는 메인 핸들러.
///
/// ## 처리 흐름
/// 1. 활성(파산하지 않은) 플레이어 목록을 조회한다.
/// 2. 현재 턴 플레이어가 주사위를 굴려 이동한다.
/// 3. 도착한 타일에 따라 분기 처리:
///    - **구매 가능 타일**: 이동·월급만 선반영 후 `PendingTurn` 설정 → 클라이언트에 구매 여부 질의
///    - **그 외 타일**: 통행료 지불, 이벤트 처리 등을 즉시 실행
/// 4. 턴 종료 후 게임 종료 여부를 판단한다.
pub fn handle_turn(conn: &Connection, session: &mut SessionState) -> rusqlite::Result<ApiTurnResponse> {
    // 파산하지 않은 활성 플레이어만 조회
    let players = get_all_players(conn)?
        .into_iter()
        .filter(|player| !player.is_bankrupt)
        .map(|row| to_game_player(&row))
        .collect::<Vec<_>>();

    // 활성 플레이어가 없으면 게임 즉시 종료
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

    // 턴 인덱스가 범위를 벗어나면 0으로 초기화 (라운드 시작)
    if session.current_turn_index >= players.len() {
        session.current_turn_index = 0;
    }

    let current_player = &players[session.current_turn_index];

    // 주사위 굴리기 → 새 위치·랩·월급 계산 (보드 크기: 24칸)
    let move_step = roll_and_move(current_player.position, current_player.lap, 24);

    // 도착 타일의 정보(가격, 통행료, 타입)와 소유자 조회
    let (tile_price, tile_toll, _, tile_type) =
        get_tile_info(conn, move_step.new_position).unwrap_or((0, 0, None, String::from("unknown")));
    let tile_owner = get_owner(conn, move_step.new_position).unwrap_or(None);
    let money_after_salary = current_player.money + move_step.salary;

    // ── 분기 1: 구매 가능 타일 (소유자 없음) → 구매 여부를 클라이언트에 질의 ──
    if is_purchasable_tile(tile_owner, &tile_type, tile_price) {
        // 이동 + 월급만 먼저 DB에 반영 (구매 결정은 아직 미반영)
        pre_apply_move_salary(conn, current_player.id, move_step.new_position, move_step.new_lap, move_step.salary)?;

        // 구매 결정 대기 상태 저장 → 이후 handle_decide()에서 처리
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

    // ── 분기 2: 구매 불가 타일 (통행료 / 이벤트 / 빈 타일) → 턴 즉시 완료 ──
    let old_position = current_player.position;
    let old_lap = current_player.lap;
    let player_id = current_player.id;

    // 턴 결과(액션 종류, 금액 변동 등)를 계산하고 DB에 반영
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

    // 다음 턴으로 진행 및 게임 종료 여부 판단
    advance_turn(conn, session, player_id)?;

    // 갱신된 상태를 DB에서 다시 조회
    let players_after = get_player_states(conn)?;
    let tile_owners = map_tile_owners(conn)?;
    let current_player_id = current_player_id(&players_after, session.current_turn_index);

    // TurnAction 열거형을 API 응답용 문자열·숫자로 매핑
    let (action_type, action_amount, owner_id) = match &turn_result.action {
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

/// 구매 가능 타일에 대한 플레이어의 구매/패스 결정을 처리하고 턴을 완료한다.
///
/// `handle_turn`에서 `PendingTurn`이 설정된 경우에만 호출해야 한다.
/// `will_buy`가 `true`이면 타일을 구매하고, `false`이면 구매를 건너뛴다.
pub fn handle_decide(
    conn: &Connection,
    session: &mut SessionState,
    will_buy: bool,
) -> rusqlite::Result<ApiTurnResponse> {
    // 대기 중인 구매 결정이 없으면 에러 반환
    let pending = match session.pending.take() {
        Some(p) => p,
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    };

    // 구매 여부에 따른 결과 계산
    let buy_result = decide_buy_property(
        pending.player_id,
        pending.money_after_salary,
        pending.tile_price,
        0,
        None,
        will_buy,
        "property".to_string(),
    );

    // 구매 시 DB에 소유권·금액 반영, 미구매 시 건너뛰기
    let (action_type, action_amount) = match &buy_result {
        BuyResult::Purchase { price } => {
            apply_purchase(conn, pending.player_id, pending.new_position, *price)?;
            ("purchase", *price)
        }
        _ => ("skip", 0),
    };

    // 다음 턴으로 진행 및 게임 종료 여부 판단
    advance_turn(conn, session, pending.player_id)?;

    let players_after = get_player_states(conn)?;
    let tile_owners = map_tile_owners(conn)?;
    let current_player_id = current_player_id(&players_after, session.current_turn_index);

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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  턴 진행 내부 로직
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 다음 턴으로 진행하고 게임 종료 조건을 판단한다.
///
/// - 게임 종료 조건 충족 시: 보상을 DB에 반영하고 `session`에 결과를 기록한다.
/// - 아직 진행 중이면: 턴 인덱스를 1 증가시킨다.
fn advance_turn(
    conn: &Connection,
    session: &mut SessionState,
    _player_id: i32,
) -> rusqlite::Result<()> {
    let all_rows = get_all_players(conn)?;
    let game_players: Vec<GamePlayer> = all_rows.iter().map(to_game_player).collect();

    // 게임 종료 조건 확인 (전원 파산, 랩 초과 등)
    let game_result = check_game_end(game_players);
    session.game_finished = game_result.is_finished;

    if session.game_finished {
        // 게임 종료: 순위별 보상금을 DB에 반영
        apply_rewards(conn, &game_result.rewards)?;

        session.winner_id = game_result.winner_id;
        session.final_rankings = Some(game_result.rankings);
    } else {
        // 게임 진행 중: 다음 플레이어에게 턴 넘김
        session.current_turn_index += 1;
        session.winner_id = None;
        session.final_rankings = None;
    }

    Ok(())
}