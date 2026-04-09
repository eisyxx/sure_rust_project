#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::service::event_service::{handle_event, handle_event_with_repo, EventResult};
    use crate::service::traits::EventServiceRepo;

    // ── Mock: DB 대신 사용할 가짜 Repo ───────────────────────────────────────
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

    // 더미 DB 커넥션 (Mock 테스트용, 실제 사용 안함)
    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    // ── 실제 DB 초기화 헬퍼 (통합 테스트용) ──────────────────────────────────
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE event_tiles (tile_id INTEGER PRIMARY KEY, event_type TEXT NOT NULL, amount INTEGER NOT NULL);
            CREATE TABLE players (id INTEGER PRIMARY KEY, money INTEGER NOT NULL);
            CREATE TABLE properties (tile_id INTEGER PRIMARY KEY, owner_id INTEGER, price INTEGER NOT NULL);
            CREATE TABLE fund (amount INTEGER NOT NULL);"
        ).unwrap();
        conn
    }
    // ══════════════════════════════════════════════════════════════════════════
    // Mock 기반 단위 테스트 (기존 유지)
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn event_info_error_returns_none() {
        let repo = MockRepo { event_info: None, player_money: None, total_property_price: None, fund_amount: None };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }

    #[test]
    fn unknown_event_type_returns_none() {
        let repo = MockRepo { event_info: Some(("unknown_type".to_string(), 10)), player_money: None, total_property_price: None, fund_amount: None };
        let result = handle_event_with_repo(&repo, &dummy_conn(), 1, 1);
        assert_eq!(result, EventResult::None);
    }

    // ── A: 사회복지기금 (fund_add) ────────────────────────
    #[test]
    fn fund_add_sufficient_money() {
        let repo = MockRepo { event_info: Some(("fund_add".to_string(), 50)), player_money: Some(100), total_property_price: None, fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::WelfareFund { amount: 50 });
    }

    #[test]
    fn fund_add_exact_money() {
        let repo = MockRepo { event_info: Some(("fund_add".to_string(), 100)), player_money: Some(100), total_property_price: None, fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::WelfareFund { amount: 100 });
    }

    #[test]
    fn fund_add_insufficient_money() {
        let repo = MockRepo { event_info: Some(("fund_add".to_string(), 50)), player_money: Some(30), total_property_price: None, fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::WelfareFundBankrupt { paid: 30 });
    }

    #[test]
    fn fund_add_money_error_returns_none() {
        let repo = MockRepo { event_info: Some(("fund_add".to_string(), 50)), player_money: None, total_property_price: None, fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::None);
    }

    // ── B: 종합부동산세 (tax_if_property) ──────────────────
    #[test]
    fn tax_property_sufficient_money() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 30)), player_money: Some(200), total_property_price: Some(150), fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::EstateTax { amount: 30 });
    }

    #[test]
    fn tax_property_exact_boundary() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 30)), player_money: Some(30), total_property_price: Some(100), fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::EstateTax { amount: 30 });
    }

    #[test]
    fn tax_property_insufficient_money() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 50)), player_money: Some(20), total_property_price: Some(200), fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::EstateTaxBankrupt { paid: 20 });
    }

    #[test]
    fn tax_property_skipped_low_total() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 30)), player_money: Some(200), total_property_price: Some(50), fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::EstateTaxSkipped);
    }

    #[test]
    fn tax_property_skipped_when_total_just_below() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 30)), player_money: Some(200), total_property_price: Some(99), fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::EstateTaxSkipped);
    }

    #[test]
    fn tax_property_price_error_skipped() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 30)), player_money: Some(200), total_property_price: None, fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::EstateTaxSkipped);
    }

    #[test]
    fn tax_property_money_error_returns_none() {
        let repo = MockRepo { event_info: Some(("tax_if_property".to_string(), 30)), player_money: None, total_property_price: Some(200), fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::None);
    }

    // ── C: 기금 수령 (fund_take) ──────────────────────────
    #[test]
    fn fund_take_with_fund() {
        let repo = MockRepo { event_info: Some(("fund_take".to_string(), 0)), player_money: None, total_property_price: None, fund_amount: Some(500) };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::FundReceive { amount: 500 });
    }

    #[test]
    fn fund_take_empty() {
        let repo = MockRepo { event_info: Some(("fund_take".to_string(), 0)), player_money: None, total_property_price: None, fund_amount: Some(0) };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::FundReceiveEmpty);
    }

    #[test]
    fn fund_take_error_returns_none() {
        let repo = MockRepo { event_info: Some(("fund_take".to_string(), 0)), player_money: None, total_property_price: None, fund_amount: None };
        assert_eq!(handle_event_with_repo(&repo, &dummy_conn(), 1, 1), EventResult::None);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // 실제 DB 기반 통합 테스트 (impl 함수 커버리지 확보용)
    // ══════════════════════════════════════════════════════════════════════════

    /// fund_add → get_event_info + get_player_money 실행
    #[test]
    fn integration_fund_add() {
        let conn = setup_test_db();
        conn.execute("INSERT INTO event_tiles (tile_id, event_type, amount) VALUES (1, 'fund_add', 50)", []).unwrap();
        conn.execute("INSERT INTO players (id, money) VALUES (1, 200)", []).unwrap();

        let result = handle_event(&conn, 1, 1);
        assert!(matches!(result, EventResult::WelfareFund { amount: 50 }));
    }

    /// tax_if_property → get_event_info + get_player_total_property_price + get_player_money 실행
    #[test]
    fn integration_tax_if_property() {
        let conn = setup_test_db();
        conn.execute("INSERT INTO event_tiles (tile_id, event_type, amount) VALUES (2, 'tax_if_property', 30)", []).unwrap();
        conn.execute("INSERT INTO players (id, money) VALUES (1, 200)", []).unwrap();
        conn.execute("INSERT INTO properties (tile_id, owner_id, price) VALUES (10, 1, 150)", []).unwrap();

        let result = handle_event(&conn, 1, 2);
        assert!(matches!(result, EventResult::EstateTax { amount: 30 }));
    }

    /// fund_take → get_event_info + get_fund_amount 실행
    #[test]
    fn integration_fund_take() {
        let conn = setup_test_db();
        conn.execute("INSERT INTO event_tiles (tile_id, event_type, amount) VALUES (3, 'fund_take', 0)", []).unwrap();
        conn.execute("INSERT INTO fund (amount) VALUES (500)", []).unwrap();
        
        let result = handle_event(&conn, 1, 3);
        assert!(matches!(result, EventResult::FundReceive { amount: 500 }));
    }
}
