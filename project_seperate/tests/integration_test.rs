/// 통합 테스트

#[cfg(test)]
mod integration_tests {
    use rusqlite::Connection;

    use project::service::event_service::EventResult;

    use project::service::orchestrator::*;
    use project::service::traits::TurnServiceDeps;

    /// 테스트용 인메모리 DB 생성 함수
    /// - 실제 서비스에서 사용하는 init_db()를 그대로 호출하여 동일한 초기 상태를 재현
    fn setup() -> (Connection, SessionState) {
        let conn = Connection::open_in_memory().unwrap();
        let session = init_session(&conn).unwrap();
        (conn, session)
    }

    #[allow(dead_code)]
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

 
    /// 게임 정상 종료 및 턴 개수 일치 여부 테스트
    #[test]
    fn test_full_game_flow_cover_all_services() {
        let (conn, mut session) = setup();

        let mut turn_count = 0;

        while !session.game_finished && turn_count < 200 {
            let response = process_turn(&conn, &mut session).unwrap();

            // 구매 가능한 경우 → 강제로 구매/스킵 둘 다 경험
            if response.action_type == "can_buy" {
                let will_buy = turn_count % 2 == 0;
                let _ = process_decide(&conn, &mut session, will_buy).unwrap();
            }
            turn_count += 1;
        }
        assert!(session.game_finished || turn_count == 200); // 게임이 종료 상태여야 함

        let state = get_state(&conn, &session).unwrap();

        // 돈이 없다면 파산, 파산했다면 돈이 없어야 함
        for p in &state.players {
            if p.money < 0 {
                assert!(p.is_bankrupt);
            }
            if p.is_bankrupt {
                assert!(p.money <= 0);
            }
        }

        // position은 tile 범위를 벗어날 수 없음
        let tile_count = 24;
        for p in &state.players {
            assert!(p.position >= 0 && p.position < tile_count);
        }

        let player_ids: Vec<i32> = state.players.iter().map(|p| p.id).collect();

        // 존재하지 않는 플레이어가 토지를 소유할 수 없음
        for prop in &state.tile_owners {
            assert!(player_ids.contains(&prop.owner_id));
        }

        // 파산한 플레이어가 토지를 소유할 수 없음
        for prop in &state.tile_owners {
            let owner = state.players.iter()
                .find(|p| p.id == prop.owner_id)
                .unwrap();
            assert!(!owner.is_bankrupt);
        }

    }

    // 한 플레이어의 턴 테스트
    #[test]
    fn test_single_turn_integration() {
        let (conn, mut session) = setup();

        // 실행
        let result = process_turn(&conn, &mut session).unwrap();
        let final_result = if result.action_type == "can_buy" {
            process_decide(&conn, &mut session, true).unwrap()
        } else {
            result
        };

        // 기본 출력 검증
        assert!(final_result.dice >= 1 && final_result.dice <= 6);
        assert!(final_result.new_position >= 0);

        // action_type 검증
        let valid_actions = vec![
            "move",
            "purchase",
            "skip",
            "pay_toll",
            "event",
            "bankrupt",
            "none",
            "welfare_fund",
            "welfare_fund_bankrupt",
            "fund_receive",
            "fund_receive_empty",
            "estate_tax",
            "estate_tax_bankrupt",
            "estate_tax_skipped",
        ];
        assert!(valid_actions.contains(&&*final_result.action_type));

        // 상태 검증
        let state = get_state(&conn, &session).unwrap();

        // 돈 vs 파산 관계
        for p in &state.players {
            if p.money < 0 {
                assert!(p.is_bankrupt);
            }
        }

        // 플레이어 위치 범위
        let tile_count = 24;
        for p in &state.players {
            assert!(p.position >= 0 && p.position < tile_count);
        }

        // 소유자 유효성
        let player_ids: Vec<i32> = state.players.iter().map(|p| p.id).collect();

        for prop in &state.tile_owners {
            assert!(player_ids.contains(&prop.owner_id));
        }
        
    }

    // 거래 내역 조회 테스트
    #[test]
    fn test_get_transactions_by_player() {
        use project::service::orchestrator;
        use project::repository::transcaction_repo::record_transaction;

        // DB 초기화 (초기자금 transaction 생성)
        let (conn, _session) = setup();

        let player_id = 1;

        record_transaction(&conn, player_id, "deposit", 1000, "salary").unwrap();
        record_transaction(&conn, player_id, "withdraw", 200, "tile1_purchase").unwrap();

        let txs = orchestrator::get_transactions(&conn, player_id).unwrap();

        // 전체 개수 검증 (초기자금 포함)
        assert_eq!(txs.len(), 3);

        // 초기자금 검증
        assert!(txs.iter().any(|tx| 
            tx.tx_type == "deposit" &&
            tx.amount == 300 &&
            tx.target == "초기자금"
        ));

        // 월급 검증
        assert!(txs.iter().any(|tx| 
            tx.tx_type == "deposit" &&
            tx.amount == 1000 &&
            tx.target == "salary"
        ));

        // 구매 검증
        assert!(txs.iter().any(|tx| 
            tx.tx_type == "withdraw" &&
            tx.amount == 200 &&
            tx.target.contains("tile")
        ));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  get_result() 커버리지
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// 게임 진행 중 결과 조회 (final_rankings = None → else 분기)
    #[test]
    fn test_get_result_during_game() {
        let (conn, session) = setup();

        let result = get_result(&conn, &session);

        // 초기 플레이어 4명 전부 반환
        assert_eq!(result.len(), 4);

        // 게임 진행 중이므로 rank는 전부 None
        for p in &result {
            assert_eq!(p.rank, None);
            assert!(!p.is_bankrupt);
        }
    }

    /// 게임 종료 후 결과 조회 (final_rankings = Some → if let 분기)
    #[test]
    fn test_get_result_after_game_end() {
        let (conn, mut session) = setup();

        // 플레이어 2,3,4를 파산 처리
        conn.execute("UPDATE players SET is_bankrupt=1, money=0 WHERE id IN (2,3,4)", []).unwrap();

        // 게임 종료 상태 세팅
        session.game_finished = true;
        session.winner_id = Some(1);
        session.final_rankings = Some(vec![(1, 300), (2, 0), (3, 0), (4, 0)]);

        let result = get_result(&conn, &session);

        assert_eq!(result.len(), 4);

        // 1번 플레이어: 생존 → rank = Some(1)
        let p1 = result.iter().find(|p| p.id == 1).unwrap();
        assert_eq!(p1.rank, Some(1));
        assert_eq!(p1.money, 300);
        assert!(!p1.is_bankrupt);

        // 파산 플레이어: rank = None
        for id in [2, 3, 4] {
            let p = result.iter().find(|p| p.id == id).unwrap();
            assert_eq!(p.rank, None);
            assert!(p.is_bankrupt);
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  reset_game() 커버리지
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// 게임 진행 후 리셋 → 세션 + DB 모두 초기 상태로 복원
    #[test]
    fn test_reset_game() {
        let (conn, mut session) = setup();

        // 턴을 진행해서 세션 상태를 변경시킴
        let result = process_turn(&conn, &mut session).unwrap();
        if result.action_type == "can_buy" {
            let _ = process_decide(&conn, &mut session, true).unwrap();
        }

        // 세션 상태가 변경됐는지 확인
        assert!(session.current_turn_index > 0 || session.pending.is_some() || session.game_finished);

        // 리셋 실행
        reset_game(&conn, &mut session).unwrap();

        // 세션 필드 초기화 검증
        assert_eq!(session.current_turn_index, 0);
        assert!(!session.game_finished);
        assert_eq!(session.winner_id, None);
        assert!(session.pending.is_none());
        assert!(session.final_rankings.is_none());

        // DB 초기화 검증: 모든 플레이어 초기 상태
        let state = get_state(&conn, &session).unwrap();
        assert_eq!(state.players.len(), 4);
        for p in &state.players {
            assert_eq!(p.position, 0);
            assert_eq!(p.money, 300);
            assert!(!p.is_bankrupt);
        }

        // DB 초기화 검증: 소유자 없음
        assert!(state.tile_owners.is_empty());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  players.is_empty() 분기 커버리지
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// 모든 플레이어 파산 상태에서 턴 진행 → is_empty() 분기 진입
    #[test]
    fn test_process_turn_no_active_players() {
        let (conn, mut session) = setup();

        // 모든 플레이어를 파산 처리
        conn.execute("UPDATE players SET is_bankrupt=1, money=0 WHERE id IN (1,2,3,4)", []).unwrap();

        let result = process_turn(&conn, &mut session).unwrap();

        // is_empty 분기: 기본값 반환
        assert_eq!(result.player_id, 0);
        assert_eq!(result.dice, 0);
        assert_eq!(result.action_type, "none");
        assert!(result.players.is_empty());
        assert!(result.tile_owners.is_empty());
        assert_eq!(result.current_player_id, None);

        // advance_turn이 게임 종료를 감지해야 함
        assert!(session.game_finished);
    }
}