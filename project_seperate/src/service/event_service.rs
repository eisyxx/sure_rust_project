use rusqlite::Connection;
use crate::service::traits::EventServiceRepo;
use crate::service::port_impl::PortImpl;


/// 이벤트 결과
#[derive(Debug, PartialEq)]
pub enum EventResult {
    WelfareFund { amount: i32 },
    WelfareFundBankrupt { paid: i32 },
    EstateTax { amount: i32 },
    EstateTaxBankrupt { paid: i32 }, 
    EstateTaxSkipped,
    FundReceive { amount: i32 },
    FundReceiveEmpty,
    None,
}

pub fn handle_event_with_repo<R: EventServiceRepo>(
    repo: &R,
    conn: &Connection,
    player_id: i32,
    tile_id: i32,
) -> EventResult {
    let (event_type, amount) = match repo.get_event_info(conn, tile_id) {
        Ok(info) => info,
        Err(_) => return EventResult::None,
    };

    match event_type.as_str() {

        // A: 사회복지기금
        "fund_add" => {
            let current_money = match repo.get_player_money(conn, player_id) {
                Ok(m) => m,
                Err(_) => return EventResult::None,
            };

            if current_money >= amount {
                EventResult::WelfareFund { amount }
            } else {
                EventResult::WelfareFundBankrupt { paid: current_money }
            }
        }

        // B: 종합부동산세
        "tax_if_property" => {
            let total = repo
                .get_player_total_property_price(conn, player_id)
                .unwrap_or(0);

            if total >= 100 {
                let current_money = match repo.get_player_money(conn, player_id) {
                    Ok(m) => m,
                    Err(_) => return EventResult::None,
                };

                if current_money >= amount {
                    EventResult::EstateTax { amount }
                } else {
                    EventResult::EstateTaxBankrupt { paid: current_money }
                }
            } else {
                EventResult::EstateTaxSkipped
            }
        }

        // C: 기금 수령
        "fund_take" => {
            let fund_amount = match repo.get_fund_amount(conn) {
                Ok(a) => a,
                Err(_) => return EventResult::None,
            };

            if fund_amount > 0 {
                EventResult::FundReceive { amount: fund_amount }
            } else {
                EventResult::FundReceiveEmpty
            }
        }

        _ => EventResult::None,
    }
}

/// 이벤트 처리
pub fn handle_event(
    conn: &Connection,
    player_id: i32,
    tile_id: i32,
) -> EventResult {
    handle_event_with_repo(&PortImpl, conn, player_id, tile_id)
}
