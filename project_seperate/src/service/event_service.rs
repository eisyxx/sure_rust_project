use rusqlite::Connection;

use crate::repository::{
    event_repo,
    player_repo,
    property_repo,
};

/// 이벤트 결과
#[derive(Debug, PartialEq, Eq)]
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

/// Repository 추상화 trait
pub trait EventRepository {
    fn get_event_info(&self, tile_id: i32) -> rusqlite::Result<(String, i32)>;
    fn get_player_money(&self, player_id: i32) -> rusqlite::Result<i32>;
    fn get_player_total_property_price(&self, player_id: i32) -> rusqlite::Result<i32>;
    fn get_fund_amount(&self) -> rusqlite::Result<i32>;
}

/// Production 용 adapter
pub struct DbEventRepository<'a> {
    conn: &'a Connection,
}

impl<'a> DbEventRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        DbEventRepository { conn }
    }
}

impl<'a> EventRepository for DbEventRepository<'a> {
    fn get_event_info(&self, tile_id: i32) -> rusqlite::Result<(String, i32)> {
        event_repo::get_event_info(self.conn, tile_id)
    }

    fn get_player_money(&self, player_id: i32) -> rusqlite::Result<i32> {
        player_repo::get_player_money(self.conn, player_id)
    }

    fn get_player_total_property_price(&self, player_id: i32) -> rusqlite::Result<i32> {
        property_repo::get_player_total_property_price(self.conn, player_id)
    }

    fn get_fund_amount(&self) -> rusqlite::Result<i32> {
        event_repo::get_fund_amount(self.conn)
    }
}

/// 이벤트 처리
pub fn handle_event<R: EventRepository>(
    repo: &R,
    player_id: i32,
    tile_id: i32,
) -> EventResult {
    let (event_type, amount) = match repo.get_event_info(tile_id) {
        Ok(info) => info,
        Err(_) => return EventResult::None,
    };

    match event_type.as_str() {

        // A: 사회복지기금
        "fund_add" => {
            let current_money = match repo.get_player_money(player_id) {
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
            let total = repo.get_player_total_property_price(player_id)
                .unwrap_or(0);

            if total >= 100 {
                let current_money = match repo.get_player_money(player_id) {
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
            let fund_amount = match repo.get_fund_amount() {
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

/// Connection을 받는 래퍼 함수 (기존 코드와의 호환성)
pub fn handle_event_with_conn(
    conn: &Connection,
    player_id: i32,
    tile_id: i32,
) -> EventResult {
    let repo = DbEventRepository::new(conn);
    handle_event(&repo, player_id, tile_id)
}