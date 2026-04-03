use rusqlite::Connection;

use crate::repository::player_repo::{
    get_all_players, get_player_states, PlayerState,
};
use crate::repository::{property_repo::get_owner, tile_repo::get_tile_info};
use crate::service::{
    movement_service::move_player,
    salary_service::calculate_salary,
    buy_property_service::{decide_buy_property, BuyResult},
    traits::PlayerStateRepo,
    roll_dice_service::roll_dice,
    event_service::{handle_event, EventResult},
    game_end_service::Player as GamePlayer,
    traits::TurnServiceDeps,
};

// 한 턴 진행 결과 데이터
pub struct TurnResult {
    pub dice: i32,
    pub new_position: i32,
    pub new_lap: i32,
    pub salary: i32,
    pub action: TurnAction,
}

// 이동만 처리한 중간 결과 (구매 결정 전 단계)
pub struct MoveStep {
    pub dice: i32,
    pub new_position: i32,
    pub new_lap: i32,
    pub salary: i32,
}

/// 이동 후 착지 타일 판정에 필요한 컨텍스트.
pub struct LandingContext {
    pub tile_price: i32,
    pub tile_toll: i32,
    pub tile_owner: Option<i32>,
    pub tile_type: String,
    pub money_after_salary: i32,
}

pub struct TurnServiceDepsImpl;

impl TurnServiceDeps for TurnServiceDepsImpl {
    fn roll_dice(&self) -> i32 {
        roll_dice()
    }
    fn handle_event(&self, conn: &Connection, player_id: i32, tile_id: i32) -> EventResult {
        handle_event(conn, player_id, tile_id)
    }
}

pub fn roll_and_move_with_deps<D: TurnServiceDeps>(
    deps: &D,
    position: i32,
    lap: i32,
    total_tiles: i32,
) -> MoveStep {
    let dice = deps.roll_dice();
    let move_result = move_player(position, lap, dice, total_tiles);
    let salary = calculate_salary(lap, move_result.new_lap, 20);
    MoveStep {
        dice,
        new_position: move_result.new_position,
        new_lap: move_result.new_lap,
        salary,
    }
}

/// 도착 타일 정보와 소유자 조회 + 월급 반영 후 잔액 계산을 한 번에 수행한다.
pub fn build_landing_context(
    conn: &Connection,
    new_position: i32,
    current_money: i32,
    salary: i32,
) -> LandingContext {
    let (tile_price, tile_toll, _, tile_type) =
        get_tile_info(conn, new_position).unwrap_or((0, 0, None, String::from("unknown")));
    let tile_owner = get_owner(conn, new_position).unwrap_or(None);

    LandingContext {
        tile_price,
        tile_toll,
        tile_owner,
        tile_type,
        money_after_salary: current_money + salary,
    }
}

pub fn build_turn_result_with_deps<D: TurnServiceDeps>(
    deps: &D,
    conn: &Connection,
    move_step: MoveStep,
    player_id: i32,
    money_after_salary: i32,
    tile_price: i32,
    tile_toll: i32,
    tile_owner: Option<i32>,
    tile_type: &str,
) -> TurnResult {
    let action = if tile_type == "event" {
        match deps.handle_event(conn, player_id, move_step.new_position) {
            EventResult::WelfareFund { amount } => TurnAction::EventWelfareFund { amount },
            EventResult::WelfareFundBankrupt { paid } => TurnAction::EventWelfareFundBankrupt { paid },
            EventResult::EstateTax { amount } => TurnAction::EstateTax { amount },
            EventResult::EstateTaxBankrupt { paid } => TurnAction::EstateTaxBankrupt { paid },
            EventResult::EstateTaxSkipped => TurnAction::EstateTaxSkipped,
            EventResult::FundReceive { amount } => TurnAction::EventFundReceive { amount },
            EventResult::FundReceiveEmpty => TurnAction::FundReceiveEmpty,
            EventResult::None => TurnAction::None,
        }
    } else {
        let buy_result = decide_buy_property(
            player_id,
            money_after_salary,
            tile_price,
            tile_toll,
            tile_owner,
            false,
            tile_type.to_string(),
        );
        match buy_result {
            BuyResult::PayToll { owner_id, amount } => TurnAction::PayToll { owner_id, amount },
            BuyResult::Bankrupt { owner_id, paid } => TurnAction::Bankrupt { owner_id, paid },
            BuyResult::Purchase { .. } | BuyResult::NotEnoughMoney | BuyResult::Skip => TurnAction::None,
        }
    };

    TurnResult {
        dice: move_step.dice,
        new_position: move_step.new_position,
        new_lap: move_step.new_lap,
        salary: move_step.salary,
        action,
    }
}

/// MoveStep → TurnResult 생성 (통행료/이벤트/None 처리, 구매는 process_decide 경로)
pub fn build_turn_result(
    conn: &Connection,
    move_step: MoveStep,
    player_id: i32,
    money_after_salary: i32,
    tile_price: i32,
    tile_toll: i32,
    tile_owner: Option<i32>,
    tile_type: &str,
) -> TurnResult {
    build_turn_result_with_deps(
        &TurnServiceDepsImpl, conn, move_step, player_id,
        money_after_salary, tile_price, tile_toll, tile_owner, tile_type,
    )
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  플레이어·턴 조회 서비스
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// DB에서 파산하지 않은 활성 플레이어를 `GamePlayer` 형태로 반환한다.
pub fn get_active_game_players(conn: &Connection) -> rusqlite::Result<Vec<GamePlayer>> {
    let rows = get_all_players(conn)?;
    Ok(rows
        .into_iter()
        .filter(|r| !r.is_bankrupt)
        .map(|r| GamePlayer {
            id: r.id,
            position: r.position,
            money: r.money,
            lap: r.lap,
            is_bankrupt: r.is_bankrupt,
        })
        .collect())
}

struct PlayerStateRepoImpl;

impl PlayerStateRepo for PlayerStateRepoImpl {
    fn get_player_states(&self, conn: &Connection) -> rusqlite::Result<Vec<PlayerState>> {
        get_player_states(conn)
    }
}

pub fn resolve_current_player_id_with_repo<R: PlayerStateRepo>(
    repo: &R,
    conn: &Connection,
    current_turn_index: usize,
) -> rusqlite::Result<Option<i32>> {
    let players = repo.get_player_states(conn)?;
    let active: Vec<&PlayerState> = players.iter().filter(|p| !p.is_bankrupt).collect();

    if active.is_empty() {
        return Ok(None);
    }

    let normalized = current_turn_index % active.len();
    Ok(Some(active[normalized].id))
}

/// 현재 턴 인덱스를 기반으로 실제 턴을 진행할 플레이어의 ID를 반환한다.
///
/// DB에서 플레이어 상태를 조회한 뒤, 활성(비파산) 플레이어 수로
/// 인덱스를 정규화하여 해당 플레이어 ID를 계산한다.
/// 활성 플레이어가 없으면 `None`을 반환한다.
pub fn resolve_current_player_id(conn: &Connection, current_turn_index: usize) -> rusqlite::Result<Option<i32>> {
    resolve_current_player_id_with_repo(&PlayerStateRepoImpl, conn, current_turn_index)
}

// 턴 동안 발생한 행동 종류
#[derive(Debug, PartialEq)]
pub enum TurnAction {
    None,
    PayToll { owner_id: i32, amount: i32 },
    Bankrupt { owner_id: i32, paid: i32 },
    EventWelfareFund { amount: i32 },
    EventWelfareFundBankrupt { paid: i32 },
    EventFundReceive { amount: i32 },
    FundReceiveEmpty,
    EstateTax { amount: i32 },
    EstateTaxBankrupt { paid: i32 },
    EstateTaxSkipped,
}

impl TurnAction {
    pub fn is_bankrupt(&self) -> bool {
        matches!(self, TurnAction::Bankrupt { .. } | TurnAction::EventWelfareFundBankrupt { .. } | TurnAction::EstateTaxBankrupt { .. })
    }
}