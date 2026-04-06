#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::service::event_service::{handle_event_with_repo, EventResult};
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

    // 더미 DB 커넥션 (실제 사용 안함)
    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    // ── 이벤트 정보 조회 실패 ─────────────────────────────
    #[test]
    // event_info 조회 자체가 실패하면 전체 로직을 진행하지 않고 None 반환해야 함
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
    // 정의되지 않은 이벤트 타입이면 아무 처리도 하지 않고 None 반환해야 함
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
    // 플레이어 돈이 충분한 경우 → 지정 금액만큼 정상 납부
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
    // 플레이어 돈이 정확히 납부 금액과 같은 경계값 → 정상 납부 처리
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
    // 플레이어 돈이 부족한 경우 → 가진 돈 전부 내고 파산 처리
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
    // 플레이어 돈 조회 실패 시 → 로직 진행 불가 → None 반환
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
    // 부동산 총액이 기준 이상이고, 돈도 충분 → 정상 세금 납부
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
    // 세금과 보유 금액이 정확히 같은 경계값 → 정상 납부 처리
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
    // 세금보다 돈이 부족 → 가진 돈 전부 내고 파산 처리
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
    // 부동산 총액이 기준 미만 → 세금 부과 자체를 스킵
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
    // 부동산 총액이 기준 바로 아래 경계값 → 스킵 처리 확인
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
    // 부동산 총액 조회 실패 → 안전하게 스킵 처리
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
    // 플레이어 돈 조회 실패 → 계산 자체 불가 → None 반환
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
    // 기금이 존재하는 경우 → 전액 수령
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
     // 기금이 0인 경우 → 빈 기금 처리
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
    // 기금 조회 실패 → 처리 불가 → None 반환
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