use rusqlite::Connection;

use crate::service::{
    movement_service::move_player,
    salary_service::calculate_salary,
    buy_property_service::{decide_buy_property, BuyResult},
    roll_dice_service::roll_dice,
    event_service::{handle_event_with_conn, EventResult},
};
use crate::repository::tile_repo::get_tile_info;
use crate::repository::property_repo::get_owner;

#[derive(Clone)]
// 한 턴 진행에 필요한 입력 데이터
pub struct TurnInput {
    pub player_id: i32,
    pub position: i32,
    pub lap: i32,
    pub money: i32,

    pub total_tiles: i32,

    pub tile_price: i32,
    pub tile_toll: i32,
    pub owner: Option<i32>,

    pub will_buy: bool,
    pub tile_type: String,
}

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

/// 주사위 굴리기 + 이동 + 월급 계산 수행 (구매 결정 제외)
pub fn roll_and_move(position: i32, lap: i32, total_tiles: i32) -> MoveStep {
    let dice = roll_dice();
    let move_result = move_player(position, lap, dice, total_tiles);
    let salary = calculate_salary(lap, move_result.new_lap, 20);
    MoveStep {
        dice,
        new_position: move_result.new_position,
        new_lap: move_result.new_lap,
        salary,
    }
}

/// MoveStep + 구매 여부로 TurnResult 생성 (통행료/구매/이벤트/None 처리)
pub fn build_turn_result(
    conn: &Connection,
    move_step: MoveStep,
    player_id: i32,
    money_after_salary: i32,
    tile_price: i32,
    tile_toll: i32,
    tile_owner: Option<i32>,
    will_buy: bool,
    tile_type: &str,
) -> TurnResult {
    let action = if tile_type == "event" {
        match handle_event_with_conn(conn, player_id, move_step.new_position) {
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
            will_buy,
            tile_type.to_string(),
        );
        match buy_result {
            BuyResult::PayToll { owner_id, amount } => TurnAction::PayToll { owner_id, amount },
            BuyResult::Bankrupt { owner_id, paid } => TurnAction::Bankrupt { owner_id, paid },
            BuyResult::Purchase { price } => TurnAction::Purchase { price },
            BuyResult::NotEnoughMoney | BuyResult::Skip => TurnAction::None,
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

// 턴 동안 발생한 행동 종류
#[derive(Debug)]
pub enum TurnAction {
    None,
    PayToll { owner_id: i32, amount: i32 },
    Purchase { price: i32 },
    Bankrupt { owner_id: i32, paid: i32 },
    EventWelfareFund { amount: i32 },
    EventWelfareFundBankrupt { paid: i32 },
    EventFundReceive { amount: i32 },
    FundReceiveEmpty,
    EstateTax { amount: i32 },
    EstateTaxBankrupt { paid: i32 },
    EstateTaxSkipped,
}