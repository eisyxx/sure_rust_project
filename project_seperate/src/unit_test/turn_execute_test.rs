#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use rusqlite::Connection;
    use crate::service::turn_execute_service::apply_turn_result_with_repo;
    use crate::service::turn_service::{TurnAction, TurnResult};
    use crate::service::traits::TurnExecuteRepo;

    // ── Mock ──────────────────────────────────────────────
    // DB 대신 호출 기록을 남기는 Mock Repo
    struct MockRepo {
        calls: RefCell<Vec<String>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self { calls: RefCell::new(Vec::new()) }
        }
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    // 실제 DB 작업 대신 어떤 함수가 호출됐는지만 기록
    impl TurnExecuteRepo for MockRepo {
        fn update_position_and_lap(&self, _conn: &Connection, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("update_pos_lap({},{},{})", player_id, pos, lap));
            Ok(())
        }
        fn update_money(&self, _conn: &Connection, player_id: i32, delta: i32) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("update_money({},{})", player_id, delta));
            Ok(())
        }
        fn record_transaction(&self, _conn: &Connection, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("record_tx({},{},{},{})", player_id, tx_type, amount, target));
            Ok(())
        }
        fn reset_owner_for_player(&self, _conn: &Connection, player_id: i32) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("reset_owner({})", player_id));
            Ok(())
        }
        fn bankrupt(&self, _conn: &Connection, player_id: i32) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("bankrupt({})", player_id));
            Ok(())
        }
        fn add_fund(&self, _conn: &Connection, amount: i32) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("add_fund({})", amount));
            Ok(())
        }
        fn reset_fund(&self, _conn: &Connection) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push("reset_fund".to_string());
            Ok(())
        }
    }

    // 인메모리 DB (실제 DB 영향 없음)
    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    // 테스트용 TurnResult 생성 헬퍼
    fn make_result(action: TurnAction, salary: i32) -> TurnResult {
        TurnResult { dice: 3, new_position: 5, new_lap: 1, salary, action }
    }

    // ── 월급 분기 ─────────────────────────────────────────
    #[test]
    fn salary_positive() {
        // 월급이 양수일 때:
        // - 위치/랩 업데이트
        // - 돈 증가
        // - 입금 트랜잭션 기록
        let repo = MockRepo::new();
        let result = make_result(TurnAction::None, 20);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert_eq!(calls[0], "update_pos_lap(1,5,1)");
        assert_eq!(calls[1], "update_money(1,20)");
        assert_eq!(calls[2], "record_tx(1,deposit,20,salary)");
        assert_eq!(calls.len(), 3);
    }

    #[test]
    fn salary_zero() {
        // 월급이 0일 때:
        // - 위치/랩 업데이트만 수행
        // - 돈 변화 및 트랜잭션 없음
        let repo = MockRepo::new();
        let result = make_result(TurnAction::None, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert_eq!(calls[0], "update_pos_lap(1,5,1)");
        assert_eq!(calls.len(), 1); // 월급 0이면 update_money/record_tx 없음
    }

    // ── PayToll ───────────────────────────────────────────
    #[test]
    fn action_pay_toll() {
        // 통행료 지불:
        // - 플레이어 돈 감소
        // - 소유자 돈 증가
        // - 각각 출금/입금 트랜잭션 기록
        let repo = MockRepo::new();
        let result = make_result(TurnAction::PayToll { owner_id: 2, amount: 10 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,-10)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,10,toll_to_2)".to_string()));
        assert!(calls.contains(&"update_money(2,10)".to_string()));
        assert!(calls.contains(&"record_tx(2,deposit,10,toll_from_1)".to_string()));
    }

    // ── Bankrupt ──────────────────────────────────────────
    #[test]
    fn action_bankrupt() {
        // 파산 처리:
        // - 남은 돈 상대에게 이전
        // - 트랜잭션 기록
        // - 소유권 초기화
        // - 파산 상태 반영
        let repo = MockRepo::new();
        let result = make_result(TurnAction::Bankrupt { owner_id: 2, paid: 30 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,-30)".to_string()));
        assert!(calls.contains(&"update_money(2,30)".to_string()));
        assert!(calls.contains(&"record_tx(2,deposit,30,bankrupt_from_1)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,30,bankrupt_to_2)".to_string()));
        assert!(calls.contains(&"reset_owner(1)".to_string()));
        assert!(calls.contains(&"bankrupt(1)".to_string()));
    }

    // ── EventWelfareFund ──────────────────────────────────
    #[test]
    fn action_welfare_fund() {
        // 복지기금 납부:
        // - 플레이어 돈 감소
        // - 기금 증가
        // - 출금 트랜잭션 기록
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EventWelfareFund { amount: 40 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,-40)".to_string()));
        assert!(calls.contains(&"add_fund(40)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,40,welfare_fund)".to_string()));
    }

    // ── EventWelfareFundBankrupt ──────────────────────────
    #[test]
    fn action_welfare_fund_bankrupt() {
        // 복지기금 내다가 파산:
        // - 기금 증가
        // - 출금 기록
        // - 소유권 초기화
        // - 파산 처리
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EventWelfareFundBankrupt { paid: 25 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"add_fund(25)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,25,welfare_fund_bankrupt)".to_string()));
        assert!(calls.contains(&"reset_owner(1)".to_string()));
        assert!(calls.contains(&"bankrupt(1)".to_string()));
    }

    // ── FundReceiveEmpty ──────────────────────────────────
    #[test]
    fn action_fund_receive_empty() {
        // 기금 수령 이벤트인데 기금이 0:
        // - 아무 일도 일어나지 않음 (위치 업데이트만)
        let repo = MockRepo::new();
        let result = make_result(TurnAction::FundReceiveEmpty, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert_eq!(calls.len(), 1); // update_pos_lap만
    }

    // ── EventFundReceive ──────────────────────────────────
    #[test]
    fn action_fund_receive() {
        // 기금 수령:
        // - 돈 증가
        // - 입금 기록
        // - 기금 초기화
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EventFundReceive { amount: 300 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,300)".to_string()));
        assert!(calls.contains(&"record_tx(1,deposit,300,welfare_fund_receive)".to_string()));
        assert!(calls.contains(&"reset_fund".to_string()));
    }

    // ── None ──────────────────────────────────────────────
    #[test]
    fn action_none() {
        // 아무 액션 없음:
        // - 위치/랩 업데이트만 수행
        let repo = MockRepo::new();
        let result = make_result(TurnAction::None, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        assert_eq!(repo.calls().len(), 1);
    }

    // ── EstateTaxSkipped ──────────────────────────────────
    #[test]
    fn action_estate_tax_skipped() {
        // 재산세 스킵:
        // - 아무 변화 없음 (위치 업데이트만)
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EstateTaxSkipped, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        assert_eq!(repo.calls().len(), 1);
    }

    // ── EstateTax ─────────────────────────────────────────
    #[test]
    fn action_estate_tax() {
        // 재산세 납부:
        // - 돈 감소
        // - 출금 트랜잭션 기록
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EstateTax { amount: 60 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,-60)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,60,estate_tax)".to_string()));
    }

    // ── EstateTaxBankrupt ─────────────────────────────────
    #[test]
    fn action_estate_tax_bankrupt() {
        // 재산세 내다가 파산:
        // - 일부 금액 출금
        // - 출금 기록
        // - 소유권 초기화
        // - 파산 처리
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EstateTaxBankrupt { paid: 15 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,-15)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,15,estate_tax_bankrupt)".to_string()));
        assert!(calls.contains(&"reset_owner(1)".to_string()));
        assert!(calls.contains(&"bankrupt(1)".to_string()));
    }
}