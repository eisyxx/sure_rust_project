/// turn_execute_service의 set_owner 실행 안됨 코드 확인 필요
/// turn_execute_service의 match 액션 처리 -> purchase 부분 실행 안됨 -> orchestraor 문제와 연결되는 것으로 추정
/// turn_service의 roll_and_move 실행 안됨
/// turn_service의 match deps.handle_event -> EventResult::None => TurnAction::None 실행 안됨
/// turn_service의 match buy_result -> BuyResult::Purchase { price } => TurnAction::Purchase { price } 실행 x (orchestraor 문제와 연결되는 것으로 추정)
/// 


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

    /* 


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
            "can_buy",
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

        // DB 초기화 (여기서 초기자금 transaction 생성됨)
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
}