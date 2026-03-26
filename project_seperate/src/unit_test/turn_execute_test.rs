#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use rusqlite::Connection;
    use crate::service::turn_execute_service::apply_turn_result_with_repo;
    use crate::service::turn_service::{TurnAction, TurnResult};
    use crate::service::traits::TurnExecuteRepo;

    // ── Mock ──────────────────────────────────────────────
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
        fn set_owner(&self, _conn: &Connection, tile_id: i32, player_id: i32, price: i32) -> rusqlite::Result<()> {
            self.calls.borrow_mut().push(format!("set_owner({},{},{})", tile_id, player_id, price));
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

    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn make_result(action: TurnAction, salary: i32) -> TurnResult {
        TurnResult { dice: 3, new_position: 5, new_lap: 1, salary, action }
    }

    // ── 월급 분기 ─────────────────────────────────────────
    #[test]
    fn salary_positive() {
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
        let repo = MockRepo::new();
        let result = make_result(TurnAction::None, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert_eq!(calls[0], "update_pos_lap(1,5,1)");
        assert_eq!(calls.len(), 1); // 월급 0이면 update_money/record_tx 없음
    }

    // ── Purchase ──────────────────────────────────────────
    #[test]
    fn action_purchase() {
        let repo = MockRepo::new();
        let result = make_result(TurnAction::Purchase { price: 50 }, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert!(calls.contains(&"update_money(1,-50)".to_string()));
        assert!(calls.contains(&"record_tx(1,withdraw,50,tile5_purchase)".to_string()));
        assert!(calls.contains(&"set_owner(5,1,50)".to_string()));
    }

    // ── PayToll ───────────────────────────────────────────
    #[test]
    fn action_pay_toll() {
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
        let repo = MockRepo::new();
        let result = make_result(TurnAction::FundReceiveEmpty, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        let calls = repo.calls();
        assert_eq!(calls.len(), 1); // update_pos_lap만
    }

    // ── EventFundReceive ──────────────────────────────────
    #[test]
    fn action_fund_receive() {
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
        let repo = MockRepo::new();
        let result = make_result(TurnAction::None, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        assert_eq!(repo.calls().len(), 1);
    }

    // ── EstateTaxSkipped ──────────────────────────────────
    #[test]
    fn action_estate_tax_skipped() {
        let repo = MockRepo::new();
        let result = make_result(TurnAction::EstateTaxSkipped, 0);
        apply_turn_result_with_repo(&repo, &dummy_conn(), 1, &result).unwrap();
        assert_eq!(repo.calls().len(), 1);
    }

    // ── EstateTax ─────────────────────────────────────────
    #[test]
    fn action_estate_tax() {
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