#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use rusqlite::Connection;
    use crate::service::turn_execute_service::{apply_turn_result_with_repo, pre_apply_move_salary, apply_purchase};
    use crate::service::turn_service::{TurnAction, TurnResult};
    use crate::service::traits::{TurnExecuteRepo};

    // ── Mock ──────────────────────────────────────────────
    // DB 대신 호출 기록을 남기는 Mock Repo
    struct MockRepo {
        calls: RefCell<Vec<String>>,
        fail_on: Option<&'static str>,
        fail_at: Option<i32>, 
        call_count: RefCell<HashMap<&'static str, i32>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                calls: RefCell::new(vec![]),
                fail_on: None,
                fail_at: None,
                call_count: RefCell::new(HashMap::new()),
            }
        }
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
        fn with_fail(fail_on: &'static str) -> Self {
            Self {
                calls: RefCell::new(vec![]),
                fail_on: Some(fail_on),
                fail_at: Some(1),
                call_count: RefCell::new(HashMap::new()),
            }
        }
        fn with_fail_at(fail_on: &'static str, fail_at: i32) -> Self {
            Self {
                calls: RefCell::new(vec![]),
                fail_on: Some(fail_on),
                fail_at: Some(fail_at),
                call_count: RefCell::new(HashMap::new()),
            }
        }
        fn should_fail(&self, name: &'static str) -> bool {
            let mut counts = self.call_count.borrow_mut();
            let count = counts.entry(name).or_insert(0);
            *count += 1;

            self.fail_on == Some(name) && self.fail_at == Some(*count)
        }
    }

    // 실제 DB 작업 대신 어떤 함수가 호출됐는지만 기록
    impl TurnExecuteRepo for MockRepo {
        fn update_position_and_lap(&self, _conn: &Connection, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()> {
            if self.fail_on == Some("update_position_and_lap") {
                return Err(rusqlite::Error::InvalidQuery);
            }
            self.calls.borrow_mut().push(format!("update_pos_lap({},{},{})", player_id, pos, lap));
            Ok(())
        }
        fn update_money(&self, _conn: &Connection, player_id: i32, delta: i32) -> rusqlite::Result<()> {
            if self.fail_on == Some("update_money") {
                return Err(rusqlite::Error::InvalidQuery);
            }
            self.calls.borrow_mut().push(format!("update_money({},{})", player_id, delta));
            Ok(())
        }
        fn record_transaction(&self, _conn: &Connection, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()> {
            if self.should_fail("record_transaction") {
                return Err(rusqlite::Error::InvalidQuery);
            }
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

    // ── pre_apply_move_salary ─────────────────────────────────

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE players (
                id INTEGER PRIMARY KEY,
                position INTEGER,
                money INTEGER,
                lap INTEGER,
                is_bankrupt INTEGER
            )",
            [],
        ).unwrap();

        conn.execute(
            "CREATE TABLE transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                player_id INTEGER,
                type TEXT,
                amount INTEGER,
                target TEXT,
                balance_before INTEGER,
                balance_after INTEGER,
                created_at TEXT
            )",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO players (id, position, money, lap, is_bankrupt)
            VALUES (1, 0, 100, 1, 0)",
            [],
        ).unwrap();

        conn
    }

    // helper
    fn get_player(conn: &Connection, id: i32) -> (i32, i32) {
        conn.query_row(
            "SELECT position, lap FROM players WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap()
    }

    fn get_money(conn: &Connection, id: i32) -> i32 {
        conn.query_row(
            "SELECT money FROM players WHERE id = ?1",
            [id],
            |row| row.get(0),
        ).unwrap()
    }

    // salary = 0 인 경우
    #[test]
    fn test_pre_apply_move_salary_no_salary() {
        let conn = setup_db();

        pre_apply_move_salary(&conn, 1, 5, 2, 0).unwrap();

        let (pos, lap) = get_player(&conn,1);
        let money = get_money(&conn, 1);

        assert_eq!(pos, 5);
        assert_eq!(lap, 2);
        assert_eq!(money, 100); // 변화 없음
    }

    // salary > 0 인 경우
    #[test]
    fn test_pre_apply_move_salary_with_salary() {
        let conn = setup_db();

        pre_apply_move_salary(&conn, 1, 3, 2, 50).unwrap();

        let (pos, lap) = get_player(&conn, 1);
        let money = get_money(&conn, 1);

        assert_eq!(pos, 3);
        assert_eq!(lap, 2);
        assert_eq!(money, 150);
    }

    // transaction 기록 검증
    #[test]
    fn test_pre_apply_move_salary_transaction_created() {
        let conn = setup_db();

        pre_apply_move_salary(&conn, 1, 4, 2, 30).unwrap();

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM transactions WHERE player_id = 1",
            [],
            |row| row.get(0),
        ).unwrap();

        assert_eq!(count, 1);
    }

    // ── apply_purchase ─────────────────────────────────

    fn setup_db_for_purchase() -> Connection {
        let conn = setup_db();

        conn.execute(
            "CREATE TABLE tiles (
                position INTEGER PRIMARY KEY,
                owner_id INTEGER,
                price INTEGER
            )",
            [],
        ).unwrap();

        conn.execute(
            "CREATE TABLE properties (
                tile_id INTEGER PRIMARY KEY,
                owner_id INTEGER,
                price INTEGER
            )",
            [],
        ).unwrap();

        conn
    }

    fn get_owner(conn: &Connection, pos: i32) -> Option<i32> {
        conn.query_row(
            "SELECT owner_id FROM properties WHERE tile_id = ?1",
            [pos],
            |row| row.get(0),
        ).unwrap()
    }

    // 정상 케이스 (전체 성공)
    #[test]
    fn test_apply_purchase_success() {
        let conn = setup_db_for_purchase();

        conn.execute(
            "INSERT INTO tiles (position, owner_id, price)
            VALUES (1, NULL, 100)",
            [],
        ).unwrap();

        apply_purchase(&conn, 1, 1, 100).unwrap();

        assert_eq!(get_money(&conn, 1), 0);
        assert_eq!(get_owner(&conn, 1), Some(1));
    }

    // record_transaction 커버 확인 (DB 검증)
    #[test]
    fn test_apply_purchase_transaction_created() {
        let conn = setup_db_for_purchase();

        conn.execute(
            "INSERT INTO tiles (position, owner_id, price)
            VALUES (1, NULL, 100)",
            [],
        ).unwrap();

        apply_purchase(&conn, 1, 1, 100).unwrap();

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM transactions WHERE player_id = 1",
            [],
            |row| row.get(0),
        ).unwrap();

        assert_eq!(count, 1);
    }

    // record_transaction 에러 발생
    #[test]
    fn test_apply_purchase_record_transaction_error() {
        let conn = setup_db_for_purchase();

        conn.execute(
            "INSERT INTO tiles (position, owner_id, price)
            VALUES (1, NULL, 100)",
            [],
        ).unwrap();

        // transactions 테이블 삭제
        conn.execute("DROP TABLE transactions", []).unwrap();

        let result = apply_purchase(&conn, 1, 1, 100);

        assert!(result.is_err());

        let money: i32 = conn.query_row(
            "SELECT money FROM players WHERE id = 1",
            [],
            |row| row.get(0),
        ).unwrap();

        // tile_price = 100
        assert_eq!(money, 0);
    }

    // ── ? 커버 ─────────────────────────────────

    // update_position_and_lap
    #[test]
    fn test_update_position_and_lap_error() {
        let conn = setup_db();
        let repo = MockRepo::with_fail("update_position_and_lap");

        let result = apply_turn_result_with_repo(
            &repo,
            &conn,
            1,
            &TurnResult {
                dice: 0,
                new_position: 5,
                new_lap: 2,
                salary: 0,
                action: TurnAction::None,
            },
        );

        assert!(result.is_err());
    }

    // salary record_transaction
    #[test]
    fn test_salary_record_transaction_error() {
        let conn = setup_db();
        let repo = MockRepo::with_fail("record_transaction");

        let result = apply_turn_result_with_repo(
            &repo,
            &conn,
            1,
            &TurnResult {
                dice: 0,
                new_position: 5,
                new_lap: 2,
                salary: 50,
                action: TurnAction::None,
            },
        );

        assert!(result.is_err());
    }

    // PayToll → record_transaction
    #[test]
    fn test_pay_toll_record_transaction_error() {
        let conn = setup_db();

        conn.execute("DROP TABLE transactions", []).unwrap();

        let repo = MockRepo::with_fail_at("record_transaction", 2);
        let result = apply_turn_result_with_repo(
            &repo, // 실제 구현체
            &conn,
            1,
            &TurnResult {
                dice: 2,
                new_position: 0,
                new_lap: 1,
                salary: 0,
                action: TurnAction::PayToll { owner_id: 2, amount: 50 },
            },
        );

        assert!(result.is_err());
    }

    // Bankrupt → 두 번째 record_transaction
    #[test]
    fn test_bankrupt_record_transaction_error() {
        let conn = setup_db();

        conn.execute("DROP TABLE transactions", []).unwrap();

        let repo = MockRepo::with_fail_at("record_transaction", 2);
        let result = apply_turn_result_with_repo(
            &repo,
            &conn,
            1,
            &TurnResult {
                dice: 2,
                new_position: 0,
                new_lap: 1,
                salary: 0,
                action: TurnAction::Bankrupt { owner_id: 2, paid: 50 },
            },
        );

        assert!(result.is_err());
    }

    // EventWelfareFund → update_money
    #[test]
    fn test_welfare_update_money_error() {
        let conn = setup_db();

        conn.execute("DROP TABLE players", []).unwrap();

        let repo = MockRepo::with_fail("update_money");
        let result = apply_turn_result_with_repo(
            &repo,
            &conn,
            1,
            &TurnResult {
                dice: 2,
                new_position: 0,
                new_lap: 1,
                salary: 0,
                action: TurnAction::EventWelfareFund { amount: 50 },
            },
        );

        assert!(result.is_err());
    }

    // EventWelfareFund → record_transaction
    #[test]
    fn test_welfare_record_transaction_error() {
        let conn = setup_db();

        conn.execute("DROP TABLE transactions", []).unwrap();

        let repo = MockRepo::with_fail("record_transaction");
        let result = apply_turn_result_with_repo(
            &repo,
            &conn,
            1,
            &TurnResult {
                dice: 2,
                new_position: 0,
                new_lap: 1,
                salary: 0,
                action: TurnAction::EventWelfareFund { amount: 50 },
            },
        );

        assert!(result.is_err());
    }

    // EventWelfareFundBankrupt → record_transaction
    #[test]
    fn test_welfare_bankrupt_record_error() {
        let conn = setup_db();

        conn.execute("DROP TABLE transactions", []).unwrap();

        let repo = MockRepo::with_fail("record_transaction");
        let result = apply_turn_result_with_repo(
            &repo,
            &conn,
            1,
            &TurnResult {
                dice: 2,
                new_position: 0,
                new_lap: 1,
                salary: 0,
                action: TurnAction::EventWelfareFundBankrupt { paid: 50 },
            },
        );

        assert!(result.is_err());
    }

}

