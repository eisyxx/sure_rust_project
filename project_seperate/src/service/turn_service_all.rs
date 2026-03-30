use rusqlite::Connection;
use serde::Serialize;

use crate::repository::player_repo::PlayerState;
use crate::repository::{player_repo, tile_repo, property_repo, transcaction_repo};
use crate::service::game_end_service::{check_game_end, Player};

use crate::service::{
    roll_dice_service::roll_dice,
    movement_service::move_player,
    salary_service::calculate_salary,
    buy_property_service::{decide_buy_property, BuyResult},
    event_service::{handle_event, EventResult},
};

pub mod model {
    pub use super::{TurnState, PendingDecision, SessionState, TurnInput, TurnResult};
}


/// 상태 정의
#[derive(Clone, Debug, Serialize)]
pub enum TurnState {
    Start,
    WaitingDecision(PendingDecision),
    WaitingEnd,
    Finished,
}

/// 구매 대기 상태
#[derive(Clone, Debug, Serialize)]
pub struct PendingDecision {
    pub player_id: i32,
    pub tile_id: i32,
    pub price: i32,
}

/// 세션 상태
#[derive(Clone, Serialize)]
pub struct SessionState {
    pub current_turn_index: usize,
    pub turn_state: TurnState,
    pub game_finished: bool,
    pub winner_id: Option<i32>, 
    pub final_rankings: Option<Vec<(i32,i32)>>,
}

/// 입력 정의
#[derive(Debug)]
pub enum TurnInput {
    RollDice,
    Decide { will_buy: bool },
    EndTurn,
}

/// 결과 정의
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum TurnResult {
    Moved {
        player_id: i32,
        new_position: i32,
        dice: i32,
        players: Vec<PlayerState>,
        action_type: Option<String>,
        action_amount: Option<i32>,
        salary: Option<i32>,
        game_finished: bool,
        winner_id: Option<i32>,
    },
    NeedDecision {
        player_id: i32,
        tile_id: i32,
        price: i32,
        players: Vec<PlayerState>,
        action_type: Option<String>,
        action_amount: Option<i32>,
        salary: Option<i32>,
        game_finished: bool,
        winner_id: Option<i32>,
        dice: i32,          // 새로 추가
        new_position: i32,
    },
    Completed {
        action_type: Option<String>,
        action_amount: Option<i32>,
        salary: Option<i32>,
        game_finished: bool,
        winner_id: Option<i32>,
    },
}

/// 메인 진입점
pub fn execute_turn(
    conn: &Connection,
    session: &mut SessionState,
    input: TurnInput,
) -> Result<TurnResult, String> {
    println!("🔥 현재 상태: {:?}, 입력: {:?}", session.turn_state, input); //디버깅용

    match (session.turn_state.clone(), input) {

        /// 턴 시작 → 이동
        (TurnState::Start, TurnInput::RollDice) => {

            let players = player_repo::get_all_players(conn)
                .map_err(|e| e.to_string())?;

            let active_players: Vec<_> = players
                .into_iter()
                .filter(|p| !p.is_bankrupt)
                .collect();

            if active_players.is_empty() {
                return Err("no active players".to_string());
            }

            let index = session.current_turn_index % active_players.len();
            let player = active_players.get(index).ok_or("player not found")?;

            let dice = roll_dice();

            // 이동 서비스 사용
            let move_result = move_player(
                player.position,
                player.lap,
                dice,
                24,
            );

            // DB 반영
            player_repo::update_position_and_lap(
                conn,
                player.id,
                move_result.new_position,
                move_result.new_lap,
            ).map_err(|e| e.to_string())?;

            // 월급 처리
            let salary = calculate_salary(
                player.lap,
                move_result.new_lap,
                100, // (상수 or 설정값으로 빼도 좋음)
            );

            if salary > 0 {
                player_repo::update_money(conn, player.id, salary)
                    .map_err(|e| e.to_string())?;
            }

            // 타일 정보
            let (price, toll, _, tile_type) =
                tile_repo::get_tile_info(conn, move_result.new_position)
                    .map_err(|e| e.to_string())?;

            // 이벤트 처리
            if tile_type == "event" {
                let event_result = handle_event(conn, player.id, move_result.new_position);

                match event_result {
                    EventResult::WelfareFund { amount } => {
                        player_repo::update_money(conn, player.id, -amount).map_err(|e| e.to_string())?;
                    }
                    EventResult::WelfareFundBankrupt { paid } => {
                        player_repo::update_money(conn, player.id, -paid).map_err(|e| e.to_string())?;
                        player_repo::bankrupt(conn, player.id).map_err(|e| e.to_string())?;
                    }
                    EventResult::EstateTax { amount } => {
                        player_repo::update_money(conn, player.id, -amount).map_err(|e| e.to_string())?;
                    }
                    EventResult::EstateTaxBankrupt { paid } => {
                        player_repo::update_money(conn, player.id, -paid).map_err(|e| e.to_string())?;
                        player_repo::bankrupt(conn, player.id).map_err(|e| e.to_string())?;
                    }
                    EventResult::FundReceive { amount } => {
                        player_repo::update_money(conn, player.id, amount).map_err(|e| e.to_string())?;
                    }
                    _ => {}
                }

                session.turn_state = TurnState::WaitingEnd;
                let players = player_repo::get_player_states(conn).map_err(|e| e.to_string())?;

                return Ok(TurnResult::Moved {
                    player_id: player.id,
                    new_position: move_result.new_position,
                    dice,
                    players,
                    action_type: None,
                    action_amount: None,
                    salary: Some(salary),
                    game_finished: session.game_finished,
                    winner_id: session.winner_id,
                });
            }

            // 소유자 조회
            let owner = property_repo::get_owner(conn, move_result.new_position).map_err(|e| e.to_string())?;

            // 최신 돈 조회
            let current_money = player_repo::get_player_money(conn, player.id).map_err(|e| e.to_string())?;

            // 구매/통행료 판단
            let decision = decide_buy_property(
                player.id,
                current_money,
                price,
                toll,
                owner,
                tile_type.clone(),
            );

            match decision {
                BuyResult::PayToll { owner_id, amount } => {
                    player_repo::update_money(conn, player.id, -amount).map_err(|e| e.to_string())?;
                    player_repo::update_money(conn, owner_id, amount).map_err(|e| e.to_string())?;

                    session.turn_state = TurnState::WaitingEnd;

                    let players = player_repo::get_player_states(conn).map_err(|e| e.to_string())?;

                    return Ok(TurnResult::Moved {
                        player_id: player.id,
                        new_position: move_result.new_position,
                        dice,
                        players,
                        action_type: Some("pay_toll".to_string()),
                        action_amount: Some(amount),
                        salary: Some(salary),
                        game_finished: session.game_finished,
                        winner_id: session.winner_id,
                    });
                }

                BuyResult::Bankrupt { owner_id, paid } => {
                    player_repo::update_money(conn, player.id, -paid).map_err(|e| e.to_string())?;
                    player_repo::update_money(conn, owner_id, paid).map_err(|e| e.to_string())?;
                    player_repo::bankrupt(conn, player.id).map_err(|e| e.to_string())?;

                    session.turn_state = TurnState::WaitingEnd;

                    let players = player_repo::get_player_states(conn).map_err(|e| e.to_string())?;

                    return Ok(TurnResult::Moved {
                        player_id: player.id,
                        new_position: move_result.new_position,
                        dice,
                        players,
                        action_type: Some("bankrupt".to_string()),
                        action_amount: Some(paid),
                        salary: Some(salary),
                        game_finished: session.game_finished,
                        winner_id: session.winner_id,
                    });
                }

                BuyResult::CanBuy { price } => {
                    session.turn_state = TurnState::WaitingDecision(
                        PendingDecision {
                            player_id: player.id,
                            tile_id: move_result.new_position,
                            price,
                        }
                    );

                    let players = player_repo::get_player_states(conn).map_err(|e| e.to_string())?;

                    return Ok(TurnResult::NeedDecision {
                        player_id: player.id,
                        tile_id: move_result.new_position,
                        price,
                        players,
                        action_type: Some("can_buy".to_string()),
                        action_amount: Some(price),
                        salary: Some(salary),
                        game_finished: session.game_finished,
                        winner_id: session.winner_id,
                        dice,
                        new_position: move_result.new_position,
                    });
                }

                BuyResult::Skip => {}
            }

            session.turn_state = TurnState::WaitingEnd;
            let players = player_repo::get_player_states(conn).map_err(|e| e.to_string())?;

            Ok(TurnResult::Moved {
                player_id: player.id,
                new_position: move_result.new_position,
                dice,
                players,
                action_type: None,
                action_amount: None,
                salary: Some(salary),
                game_finished: session.game_finished,
                winner_id: session.winner_id,
            })
        }

        /// 구매 선택 처리
        (TurnState::WaitingDecision(pending), TurnInput::Decide { will_buy }) => {

            let player = player_repo::get_player(conn, pending.player_id)
                .map_err(|e| e.to_string())?;

            // 구매 선택
            if will_buy {
                if player.money >= pending.price {
                    // 실제 구매 실행
                    player_repo::update_money(conn, player.id, -pending.price)
                        .map_err(|e| e.to_string())?;

                    property_repo::set_owner(conn, pending.tile_id, player.id, pending.price)
                        .map_err(|e| e.to_string())?;
                } else {
                    return Err("not enough money".to_string());
                }
            }

            // 상태 변경
            session.turn_state = TurnState::WaitingEnd;

            Ok(TurnResult::Completed {
                action_type: if will_buy {
                    Some("purchase".to_string())
                } else {
                    Some("skip_purchase".to_string())
                },
                action_amount: if will_buy {
                    Some(pending.price)
                } else {
                    None
                },
                salary: None,
                game_finished: session.game_finished,
                winner_id: session.winner_id,
            })
        }

        /// 턴 종료
        (TurnState::WaitingEnd, TurnInput::EndTurn) => {
            let db_players = player_repo::get_all_players(conn).map_err(|e| e.to_string())?;

            // game_end_service용 Player로 변환
            let players: Vec<Player> = db_players.into_iter().map(|p| Player {
                id: p.id,
                position: p.position,
                money: p.money,
                lap: p.lap,
                is_bankrupt: p.is_bankrupt,
            }).collect();

            // 게임 종료 체크
            let result = check_game_end(players);

            if result.is_finished {
                session.game_finished = true;

                // 모든 플레이어 다시 조회
                let all_players = player_repo::get_all_players(conn).map_err(|e| e.to_string())?;

                // 파산하지 않은 플레이어만
                let mut active_players: Vec<_> = all_players
                    .iter()
                    .filter(|p| !p.is_bankrupt)
                    .cloned()
                    .collect();

                // lap 기준 정렬 (내림차순)
                active_players.sort_by(|a, b| b.lap.cmp(&a.lap));

                // 상금 지급
                let rewards = [150, 120, 80];

                for (i, player) in active_players.iter().enumerate() {
                    if i < rewards.len() {
                        player_repo::give_reward(conn, player.id, rewards[i]).map_err(|e| e.to_string())?;
                    }
                }

                // 상금 지급 후 다시 조회
                let all_players_after_reward = player_repo::get_all_players(conn).map_err(|e| e.to_string())?;

                // (id, money)
                let mut final_rankings: Vec<(i32, i32)> = all_players_after_reward
                    .iter()
                    .map(|p| (p.id, p.money))
                    .collect();

                // 돈 기준 정렬
                final_rankings.sort_by(|a, b| b.1.cmp(&a.1));

                // winner 설정
                session.winner_id = final_rankings
                    .first()
                    .map(|(id, _)| *id);

                session.final_rankings = Some(final_rankings);

                return Ok(TurnResult::Completed{
                    action_type: None,
                    action_amount: None,
                    salary: None,
                    game_finished: session.game_finished,
                    winner_id: session.winner_id,
                });
            }

            // 다음 턴 진행
            session.current_turn_index += 1;
            session.turn_state = TurnState::Start;

            Ok(TurnResult::Completed{
                action_type: None,
                action_amount: None,
                salary: None,
                game_finished: session.game_finished,
                winner_id: session.winner_id,
            })
        }

        /// 잘못된 상태
        _ => Err("invalid state transition".to_string()),
    }
}
