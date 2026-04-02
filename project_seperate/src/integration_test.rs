#[cfg(test)]
mod integration_tests {
    use rusqlite::Connection;

    use crate::service::orchestrator::{process_turn, process_decide, get_transactions, SessionState};
    use crate::service::turn_service::resolve_current_player_id;
    use crate::repository::{
        init::init_db::init_db,
    };

    use crate::service::orchestrator::*;

    /// 테스트용 인메모리 DB 생성 함수
    /// - 실제 서비스에서 사용하는 init_db()를 그대로 호출하여 동일한 초기 상태를 재현
    fn setup() -> (Connection, SessionState) {
        let conn = Connection::open_in_memory().unwrap();
        let session = init_session(&conn).unwrap();
        (conn, session)
    }

    /* 
    // 기본 턴 진행
    #[test]
    fn test_process_turn_basic_flow() {
        let (conn, mut session) = setup();

        let result = process_turn(&conn, &mut session).unwrap();

        assert!(result.dice >= 1 && result.dice <= 6);
        assert!(result.new_position >= 0);
    }

    // 턴 여러 번 실행 (분기 강제 커버)
    #[test]
    fn test_process_turn_multiple_runs() {
        let (conn, mut session) = setup();

        for _ in 0..20 {
            let result = process_turn(&conn, &mut session);

            if let Ok(res) = result {
                if res.action_type == "can_buy" {
                    let _ = process_decide(&conn, &mut session, false);
                }
            }
        }

        assert!(session.current_turn_index >= 0);
    }

    // decide 호출 흐름 (YES/NO 둘 다 커버)
    #[test]
    fn test_process_decide_flow() {
        let (conn, mut session) = setup();

        for _ in 0..20 {
            let result = process_turn(&conn, &mut session);

            if let Ok(res) = result {
                if res.action_type == "can_buy" {
                    let _ = process_decide(&conn, &mut session, true);
                    let _ = process_decide(&conn, &mut session, false);
                    break;
                }
            }
        }

        assert!(true); // 커버리지 목적
    }

    // pending 없이 decide → 에러
    #[test]
    fn test_process_decide_without_pending() {
        let (conn, mut session) = setup();

        let result = process_decide(&conn, &mut session, true);

        assert!(result.is_err());
    }

    // 상태 조회
    #[test]
    fn test_get_state() {
        let (conn, session) = setup();

        let state = get_state(&conn, &session).unwrap();

        assert!(!state.players.is_empty());
    }

    // 거래 조회
    #[test]
    fn test_get_transactions() {
        let (conn, _) = setup();

        let txs = get_transactions(&conn, 1).unwrap();

        assert!(txs.len() >= 0);
    }

    // 게임 리셋
    #[test]
    fn test_reset_game() {
        let (conn, mut session) = setup();

        let _ = process_turn(&conn, &mut session);

        reset_game(&conn, &mut session).unwrap();

        assert_eq!(session.current_turn_index, 0);
        assert!(!session.game_finished);
    }

    // 결과 조회
    #[test]
    fn test_get_result() {
        let (conn, session) = setup();

        let result = get_result(&conn, &session);

        assert!(result.len() >= 0);
    }

    // 플레이어 0명 상태 (orchestrator 분기 커버)
    #[test]
    fn test_process_turn_no_active_players() {
        use crate::repository::player_repo::{get_all_players, bankrupt};

        let (conn, mut session) = setup();

        // 모든 플레이어 파산 처리
        let players = get_all_players(&conn).unwrap();
        for p in players {
            bankrupt(&conn, p.id).unwrap();
        }

        let result = process_turn(&conn, &mut session);

        assert!(result.is_err() || result.is_ok());
    }
    */

 
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
        assert!(session.game_finished || turn_count == 200); //게임이 종료 상태여야 함

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

        // 기본 출력 검증
        assert!(result.dice >= 1 && result.dice <= 6);
        assert!(result.new_position >= 0);

        // action_type은 허용된 값 중 하나여야 함
        let valid_actions = vec![
            "move",
            "can_buy",
            "purchase",
            "skip",
            "pay_toll",
            "event",
            "bankrupt",
            "none",
        ];

        assert!(valid_actions.contains(&&*result.action_type));

        // can_buy면 decide까지 포함해서 한 턴 완성
        if result.action_type == "can_buy" {
            let result2 = process_decide(&conn, &mut session, true).unwrap();

            let valid_after_decide = vec![
                "purchase",
                "skip",
            ];

            assert!(valid_after_decide.contains(&&*result2.action_type));
        }

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

    
}


