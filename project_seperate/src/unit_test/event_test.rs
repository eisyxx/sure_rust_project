#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::service::event_service::{handle_event_with_repo, EventResult};
    use crate::service::traits::EventServiceRepo;

    // ── Mock ──────────────────────────────────────────────
    struct MockRepo {
        event_info: Option<(String, i32)>,
        player_money: Option<i32>,
        total_property_price: Option<i32>,
        fund_amount: Option<i32>,
    }

    impl EventServiceRepo for MockRepo {
        fn get_event_info(&self, _conn: &Connection, _tile_id: i32) -> rusqlite::Result<(String, i32)> {
            self.event_info.clone().ok_or(rusqlite::Error::QueryReturnedNoRows)
        }
        fn get_player_money(&self, _conn: &Connection, _player_id: i32) -> rusqlite::Result<i32> {
            self.player_money.ok_or(rusqlite::Error::QueryReturnedNoRows)
        }
        fn get_player_total_property_price(&self, _conn: &Connection, _player_id: i32) -> rusqlite::Result<i32> {
            self.total_property_price.ok_or(rusqlite::Error::QueryReturnedNoRows)
        }
        fn get_fund_amount(&self, _conn: &Connection) -> rusqlite::Result<i32> {
            self.fund_amount.ok_or(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    // ── 이벤트 정보 조회 실패 ─────────────────────────────
    #[test]
    fn event_info_error_returns_none() {
        let repo = MockRepo {
            event_info: None,
            player_money: None,
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }

    // ── 알 수 없는 이벤트 타입 ────────────────────────────
    #[test]
    fn unknown_event_type_returns_none() {
        let repo = MockRepo {
            event_info: Some(("unknown_type".to_string(), 10)),
            player_money: None,
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }

    // ── A: 사회복지기금 (fund_add) ────────────────────────
    #[test]
    fn fund_add_sufficient_money() {
        let repo = MockRepo {
            event_info: Some(("fund_add".to_string(), 50)),
            player_money: Some(100),
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::WelfareFund { amount: 50 });
    }

    #[test]
    fn fund_add_exact_money() {
        let repo = MockRepo {
            event_info: Some(("fund_add".to_string(), 100)),
            player_money: Some(100),
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::WelfareFund { amount: 100 });
    }

    #[test]
    fn fund_add_insufficient_money() {
        let repo = MockRepo {
            event_info: Some(("fund_add".to_string(), 50)),
            player_money: Some(30),
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::WelfareFundBankrupt { paid: 30 });
    }

    #[test]
    fn fund_add_money_error_returns_none() {
        let repo = MockRepo {
            event_info: Some(("fund_add".to_string(), 50)),
            player_money: None,
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }

    // ── B: 종합부동산세 (tax_if_property) ──────────────────
    #[test]
    fn tax_property_sufficient_money() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 30)),
            player_money: Some(200),
            total_property_price: Some(150),
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::EstateTax { amount: 30 });
    }

    #[test]
    fn tax_property_exact_boundary() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 30)),
            player_money: Some(30),
            total_property_price: Some(100),
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::EstateTax { amount: 30 });
    }

    #[test]
    fn tax_property_insufficient_money() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 50)),
            player_money: Some(20),
            total_property_price: Some(200),
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::EstateTaxBankrupt { paid: 20 });
    }

    #[test]
    fn tax_property_skipped_low_total() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 30)),
            player_money: Some(200),
            total_property_price: Some(50),
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::EstateTaxSkipped);
    }

    #[test]
    fn tax_property_skipped_when_total_just_below() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 30)),
            player_money: Some(200),
            total_property_price: Some(99),
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::EstateTaxSkipped);
    }

    #[test]
    fn tax_property_price_error_skipped() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 30)),
            player_money: Some(200),
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::EstateTaxSkipped);
    }

    #[test]
    fn tax_property_money_error_returns_none() {
        let repo = MockRepo {
            event_info: Some(("tax_if_property".to_string(), 30)),
            player_money: None,
            total_property_price: Some(200),
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }

    // ── C: 기금 수령 (fund_take) ──────────────────────────
    #[test]
    fn fund_take_with_fund() {
        let repo = MockRepo {
            event_info: Some(("fund_take".to_string(), 0)),
            player_money: None,
            total_property_price: None,
            fund_amount: Some(500),
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::FundReceive { amount: 500 });
    }

    #[test]
    fn fund_take_empty() {
        let repo = MockRepo {
            event_info: Some(("fund_take".to_string(), 0)),
            player_money: None,
            total_property_price: None,
            fund_amount: Some(0),
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::FundReceiveEmpty);
    }

    #[test]
    fn fund_take_error_returns_none() {
        let repo = MockRepo {
            event_info: Some(("fund_take".to_string(), 0)),
            player_money: None,
            total_property_price: None,
            fund_amount: None,
        };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }
}