use rusqlite::Connection;

use crate::repository::event_repo::{get_event_info, get_fund_amount};
use crate::repository::player_repo::get_player_money;

/// 이벤트 결과
#[derive(Debug)]
pub enum EventResult {
    WelfareFund { amount: i32 },
    WelfareFundBankrupt { paid: i32 },
    FundReceive { amount: i32 },
    None,
}

/// 이벤트 처리 (결과만 반환)
pub fn handle_event(
    conn: &Connection,
    player_id: i32,
    tile_id: i32,
) -> EventResult {
    let (event_type, amount) = match get_event_info(conn, tile_id) {
        Ok(info) => info,
        Err(_) => return EventResult::None,
    };

    match event_type.as_str() {
        // A: 사회복지기금
        "A" => {
            let current_money = match get_player_money(conn, player_id) {
                Ok(m) => m,
                Err(_) => return EventResult::None,
            };

            if current_money >= amount {
                // 정상 납부
                EventResult::WelfareFund { amount }
            } else {
                // 파산
                EventResult::WelfareFundBankrupt {
                    paid: current_money,
                }
            }
        }

        // C: 기금 수령
        "C" => {
            let fund_amount = match get_fund_amount(conn) {
                Ok(a) => a,
                Err(_) => return EventResult::None,
            };

            if fund_amount > 0 {
                EventResult::FundReceive {
                    amount: fund_amount,
                }
            } else {
                EventResult::None
            }
        }

        _ => EventResult::None,
    }
}