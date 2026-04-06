#[cfg(test)]
    
mod integration_case_tests {
    use rusqlite::Connection;
    use project::service::orchestrator::*;
    use project::service::event_service::EventResult;
    use project::service::traits::TurnServiceDeps;

    #[derive(Debug)]
    struct ExpectedTurnOutcome {
        player_id: i32,
        dice: i32,
        old_position: i32,
        new_position: i32,
        old_lap: i32,
        new_lap: i32,
        salary: i32,
        action_type: &'static str,
        action_amount: i32,
        owner_id: Option<i32>,
        current_player_id: Option<i32>,
        game_finished: bool,
        winner_id: Option<i32>,
    }
    struct MockDeps {
        dice: i32,
    }

    impl TurnServiceDeps for MockDeps {
        fn roll_dice(&self) -> i32 {
            self.dice
        }
        fn handle_event(&self, _conn: &Connection, _player_id: i32, _tile_id: i32,) -> EventResult {
            EventResult::None
        }
    }

    // 인메모리 DB 생성 및 초기화
    fn setup() -> (Connection, SessionState) {
        let conn = Connection::open_in_memory().unwrap();
        let session = init_session(&conn).unwrap();
        (conn, session)
    }
    // TurnOutcome 검증 함수
    fn assert_turn_result(result: &TurnOutcome, expected: ExpectedTurnOutcome) {
        assert_eq!(result.player_id, expected.player_id);
        assert_eq!(result.dice, expected.dice);
        assert_eq!(result.old_position, expected.old_position);
        assert_eq!(result.new_position, expected.new_position);
        assert_eq!(result.old_lap, expected.old_lap);
        assert_eq!(result.new_lap, expected.new_lap);
        assert_eq!(result.salary, expected.salary);
        assert_eq!(result.action_type, expected.action_type);
        assert_eq!(result.action_amount, expected.action_amount);
        assert_eq!(result.owner_id, expected.owner_id);
        assert_eq!(result.current_player_id, expected.current_player_id);
        assert_eq!(result.game_finished, expected.game_finished);
        assert_eq!(result.winner_id, expected.winner_id);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  TRANS_NO_OWNER: 토지 소유자 없음
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// TRANS_NO_OWNER_001: 토지 소유자 없음, 구매
    #[test]
    fn trans_no_owner_001_test() {
        let (conn, mut session) = setup();

        conn.execute(
            "UPDATE players SET position=2, money=15 WHERE id=1",
            [],
        ).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;

        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        let final_result = if result.action_type == "can_buy" {
            process_decide(&conn, &mut session, true).unwrap()
        } else {
            result
        };

        assert_turn_result(&final_result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "purchase",
            action_amount: 9,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        let owner: Option<i32> = conn.query_row(
            "SELECT owner_id FROM properties WHERE tile_id=5",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(owner, Some(1));

        let money: i32 = conn.query_row(
            "SELECT money FROM players WHERE id=1", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(money, 6);
    }

    /// TRANS_NO_OWNER_002: 토지 소유자 없음, 잔액 부족으로 구매 불가
    #[test]
    fn trans_no_owner_002_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=2, money=5 WHERE id=1", []).unwrap();
        session.current_turn_index = 0;

        let repo = TurnRepoImpl;

        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        // can_buy → process_decide(true) → 잔액 부족 → skip
        let final_result = if result.action_type == "can_buy" {
            process_decide(&conn, &mut session, true).unwrap()
        } else {
            result
        };

        assert_turn_result(&final_result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "skip",
            action_amount: 0,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증: 소유자 없음, 잔액 변동 없음
        let owner: Option<i32> = conn.query_row(
            "SELECT owner_id FROM properties WHERE tile_id=5", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(owner, None);

        let money: i32 = conn.query_row(
            "SELECT money FROM players WHERE id=1", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(money, 5);
    }

    /// TRANS_NO_OWNER_003: 토지 소유자 없음, 구매 거절
    #[test]
    fn trans_no_owner_003_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=2, money=9 WHERE id=1", []).unwrap();
        session.current_turn_index = 0;

        let repo = TurnRepoImpl;

        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        // can_buy → process_decide(false) → skip
        let final_result = if result.action_type == "can_buy" {
            process_decide(&conn, &mut session, false).unwrap()
        } else {
            result
        };

        assert_turn_result(&final_result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "skip",
            action_amount: 0,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증: 소유자 없음, 잔액 변동 없음
        let owner: Option<i32> = conn.query_row(
            "SELECT owner_id FROM properties WHERE tile_id=5", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(owner, None);

        let money: i32 = conn.query_row(
            "SELECT money FROM players WHERE id=1", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(money, 9);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  TRANS_OWNER: 토지 소유자 존재
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// TRANS_OWNER_001: 토지 소유자 존재, 잔액 충분, 통행료 정상 납부
    #[test]
    fn trans_owner_001_test() {
        let (conn, mut session) = setup();

        // tile5 소유자 = player1
        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=5", []).unwrap();
        conn.execute("UPDATE players SET money=5 WHERE id=1", []).unwrap();
        conn.execute("UPDATE players SET money=10, position=2 WHERE id=2", []).unwrap();

        session.current_turn_index = 1; // player2 차례

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 2,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "pay_toll",
            action_amount: 5,
            owner_id: Some(1),
            current_player_id: Some(3),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let p2_money: i32 = conn.query_row("SELECT money FROM players WHERE id=2", [], |r| r.get(0)).unwrap();
        let p1_money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(p2_money, 5);
        assert_eq!(p1_money, 10);
    }

    /// TRANS_OWNER_002: 토지 소유자 존재, 월급 수령, 통행료 정상 납부
    #[test]
    fn trans_owner_002_test() {
        let (conn, mut session) = setup();

        // tile4 소유자 = player1
        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=4", []).unwrap();
        conn.execute("UPDATE players SET money=5 WHERE id=1", []).unwrap();
        conn.execute("UPDATE players SET money=1, position=23, lap=1 WHERE id=2", []).unwrap();

        session.current_turn_index = 1; // player2 차례

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 5 },
            &conn,
            &mut session,
        ).unwrap();

        // player2: 23+5=28, 28%24=4, lap 1→2, salary=20
        // tile4 toll=5, money_after_salary=1+20=21 >= 5 → pay_toll
        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 2,
            dice: 5,
            old_position: 23,
            new_position: 4,
            old_lap: 1,
            new_lap: 2,
            salary: 20,
            action_type: "pay_toll",
            action_amount: 5,
            owner_id: Some(1),
            current_player_id: Some(3),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증: player2.money = 1+20-5 = 16, player1.money = 5+5 = 10
        let p2_money: i32 = conn.query_row("SELECT money FROM players WHERE id=2", [], |r| r.get(0)).unwrap();
        let p1_money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let p2_lap: i32 = conn.query_row("SELECT lap FROM players WHERE id=2", [], |r| r.get(0)).unwrap();
        assert_eq!(p2_money, 16);
        assert_eq!(p1_money, 10);
        assert_eq!(p2_lap, 2);
    }

    /// TRANS_OWNER_003: 토지 소유자 존재, 잔액 부족 (파산)
    #[test]
    fn trans_owner_003_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=5", []).unwrap();
        conn.execute("UPDATE players SET money=5 WHERE id=1", []).unwrap();
        conn.execute("UPDATE players SET money=3, position=2 WHERE id=2", []).unwrap();

        session.current_turn_index = 1; // player2 차례

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        // tile5 toll=5, money=3 < 5 → bankrupt, paid=3
        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 2,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "bankrupt",
            action_amount: 3,
            owner_id: Some(1),
            current_player_id: Some(3), 
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let p2_money: i32 = conn.query_row("SELECT money FROM players WHERE id=2", [], |r| r.get(0)).unwrap();
        let p2_bankrupt: i32 = conn.query_row("SELECT is_bankrupt FROM players WHERE id=2", [], |r| r.get(0)).unwrap();
        let p1_money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(p2_money, 0);
        assert_eq!(p2_bankrupt, 1);
        assert_eq!(p1_money, 8);
    }

    /// TRANS_OWNER_004: 본인 토지 처리
    #[test]
    fn trans_owner_004_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=5", []).unwrap();
        conn.execute("UPDATE players SET money=5, position=2 WHERE id=1", []).unwrap();

        session.current_turn_index = 0; // player1 차례

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        // 본인 소유 토지 → action=none
        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "none",
            action_amount: 0,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증: 잔액 변동 없음, 소유권 유지
        let p1_money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(p1_money, 5);

        let owner: Option<i32> = conn.query_row(
            "SELECT owner_id FROM properties WHERE tile_id=5", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(owner, Some(1));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  EVENT_WELFARE: 사회복지기금 (tile 6)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// EVENT_WELFARE_ADD_001: 잔액 충분, 복지 기금 정상 납부
    #[test]
    fn event_welfare_add_001_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=2, money=100 WHERE id=1", []).unwrap();
        conn.execute("UPDATE fund SET amount=20", []).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 2,
            new_position: 6,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "welfare_fund",
            action_amount: 10,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let fund: i32 = conn.query_row("SELECT amount FROM fund", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 90);
        assert_eq!(fund, 30);
    }

    /// EVENT_WELFARE_ADD_002: 잔액 부족 (파산)
    #[test]
    fn event_welfare_add_002_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=2, money=3 WHERE id=1", []).unwrap();
        conn.execute("UPDATE fund SET amount=20", []).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 2,
            new_position: 6,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "welfare_fund_bankrupt",
            action_amount: 3,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let bankrupt: i32 = conn.query_row("SELECT is_bankrupt FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let fund: i32 = conn.query_row("SELECT amount FROM fund", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 0);
        assert_eq!(bankrupt, 1);
        assert_eq!(fund, 23);
    }

    /// EVENT_WELFARE_TAKE_001: 복지 기금 수령
    #[test]
    fn event_welfare_take_001_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=14, money=100 WHERE id=1", []).unwrap();
        conn.execute("UPDATE fund SET amount=30", []).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 14,
            new_position: 18,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "fund_receive",
            action_amount: 30,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let fund: i32 = conn.query_row("SELECT amount FROM fund", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 130);
        assert_eq!(fund, 0);
    }

    /// EVENT_WELFARE_TAKE_002: 복지 기금 수령 불가 (기금 0)
    #[test]
    fn event_welfare_take_002_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=14, money=100 WHERE id=1", []).unwrap();
        // fund = 0 (초기값)

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 14,
            new_position: 18,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "fund_receive_empty",
            action_amount: 0,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증: 잔액·기금 변동 없음
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let fund: i32 = conn.query_row("SELECT amount FROM fund", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 100);
        assert_eq!(fund, 0);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  EVENT_TAX: 종합부동산세 (tile 12)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// EVENT_TAX_001: 세금 조건 충족, 잔액 충분, 부동산 세금 정상 납부
    #[test]
    fn event_tax_001_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=8, money=100 WHERE id=1", []).unwrap();
        // total_property >= 100: tile 21(price=200) 소유
        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=21", []).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 8,
            new_position: 12,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "estate_tax",
            action_amount: 30,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 70);
    }

    /// EVENT_TAX_002: 세금 조건 충족, 잔액 부족 (파산)
    #[test]
    fn event_tax_002_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=8, money=10 WHERE id=1", []).unwrap();
        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=21", []).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 8,
            new_position: 12,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "estate_tax_bankrupt",
            action_amount: 10,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        let bankrupt: i32 = conn.query_row("SELECT is_bankrupt FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 0);
        assert_eq!(bankrupt, 1);
    }

    /// EVENT_TAX_003: 세금 조건 미충족
    #[test]
    fn event_tax_003_test() {
        let (conn, mut session) = setup();

        conn.execute("UPDATE players SET position=8, money=10 WHERE id=1", []).unwrap();
        // total_property < 100: tile 13(price=50) 소유
        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=13", []).unwrap();

        session.current_turn_index = 0;

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 4 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 4,
            old_position: 8,
            new_position: 12,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "estate_tax_skipped",
            action_amount: 0,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증: 잔액 변동 없음
        let money: i32 = conn.query_row("SELECT money FROM players WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(money, 10);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  GAME_END: 게임 종료 조건 - 생존자 1명
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// GAME_END_001: 3명 파산, 1명 생존 → 게임 종료
    #[test]
    fn game_end_001_last_survivor_test() {
        let (conn, mut session) = setup();

        // player 2, 3, 4 파산 처리
        conn.execute("UPDATE players SET is_bankrupt=1, money=0 WHERE id IN (2,3,4)", []).unwrap();
        // player 1: position=2, money=100, 본인 소유 타일로 이동 (no toll)
        conn.execute("UPDATE players SET money=100, position=2 WHERE id=1", []).unwrap();
        conn.execute("UPDATE properties SET owner_id=1 WHERE tile_id=5", []).unwrap();

        session.current_turn_index = 0; // player1 차례

        let repo = TurnRepoImpl;
        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnOutcome {
            player_id: 1,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "none",
            action_amount: 0,
            owner_id: None,
            current_player_id: Some(1),
            game_finished: true,
            winner_id: Some(1),
        });
    }
}
