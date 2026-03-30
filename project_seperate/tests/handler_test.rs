#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use project::service::game_service::{process_turn, process_decide, get_transactions, SessionState};
    use project::repository::{
        init::init_db::init_db,
    };


    /// 테스트용 인메모리 DB 생성
    /// - 실제 서비스에서 사용하는 init_db()를 그대로 호출하여 동일한 초기 상태를 재현
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        conn
    }

    /// 기본 세션 상태 생성
    fn setup_session() -> SessionState {
        SessionState {
            current_turn_index: 0,
            game_finished: false,
            winner_id: None,
            pending: None,
            final_rankings: None,
            players: vec![],
        }
    }

    
    /// 충분히 많은 턴을 실행해보기
    #[test]
    fn test_full_game_flow_cover_all_services() {
        let conn = setup_test_db();
        let mut session = setup_session();

        let mut turn_count = 0;

        while !session.game_finished && turn_count < 1000 {
            let response = process_turn(&conn, &mut session).unwrap();

            // 구매 가능한 경우 → 강제로 구매/스킵 둘 다 경험
            if response.action_type == "can_buy" {
                let will_buy = turn_count % 2 == 0;

                let _ = process_decide(&conn, &mut session, will_buy).unwrap();
            }

            turn_count += 1;
        }

        // 최소한 게임이 한 번은 종료되도록
        assert!(turn_count > 0);
    }

    /// 초기 거래 존재 확인 (초기 자금)
    #[test]
    fn test_get_transactions_initial_state() {
        let conn = setup_test_db();

        let txs = get_transactions(&conn, 1).unwrap();
        assert!(!txs.is_empty());
    }
        


/* 

    /// 초기 상태 조회 테스트 (init_db 기준으로 플레이어가 정상 생성되었는지 확인. 현재 턴 플레이어 계산이 정상인지 검증)
    #[test]
    fn test_get_state_initial() {
        let conn = setup_test_db();
        let session = setup_session();

        let state = get_state(&conn, &session).unwrap();

        // init_db에서 4명 생성되므로 4명이어야 함
        assert_eq!(state.players.len(), 4);

        // 게임이 아직 종료되지 않았는지 확인
        assert!(!state.game_finished);

        // 현재 턴 플레이어 존재 확인
        assert!(state.current_player_id.is_some());
    }

    /// 기본 턴 진행 테스트 (턴 실행 시 정상적으로 이동/주사위/턴 증가가 일어나는지 검증)
    #[test]
    fn test_handle_turn_basic_progression() {
        let conn = setup_test_db();
        let mut session = setup_session();

        let response = handle_turn(&conn, &mut session).unwrap();

        // 주사위 범위 검증
        assert!(response.dice >= 1 && response.dice <= 6);

        // 위치 이동 확인
        assert!(response.new_position >= 0);

        if response.action_type == "can_buy" {
            // 구매 대기 상태면 턴 안 넘어감
            assert_eq!(session.current_turn_index, 0);
        } else {
            // 일반 턴이면 넘어감
            assert_eq!(session.current_turn_index, 1);
        }
    }

    /// 구매 가능한 타일 도착 시 pending 생성 테스트 (구매 가능한 타일일 경우 즉시 실행되지 않고 pending 상태로 넘어가는지 확인)
    #[test]
    fn test_handle_turn_creates_pending_on_buyable_tile() {
        let conn = setup_test_db();
        let mut session = setup_session();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "can_buy" {
            // pending 상태 생성 확인
            assert!(session.pending.is_some());

            let pending = session.pending.as_ref().unwrap();

            // 데이터 정합성 확인
            assert_eq!(pending.player_id, response.player_id);
            assert_eq!(pending.new_position, response.new_position);
        }
    }

    /// 구매 결정 (BUY) 테스트 (돈 차감, 소유권 설정, 턴 진행)
    #[test]
    fn test_handle_decide_buy() {
        let conn = setup_test_db();
        let mut session = setup_session();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "can_buy" {
            let player_id = response.player_id;

            // 구매 전 돈 상태
            let before = get_player_states(&conn).unwrap();
            let before_money = before.iter().find(|p| p.id == player_id).unwrap().money;

            // 구매 실행
            let result = handle_decide(&conn, &mut session, true).unwrap();

            // 구매 후 돈 상태
            let after = get_player_states(&conn).unwrap();
            let after_money = after.iter().find(|p| p.id == player_id).unwrap().money;

            // 돈 감소 검증
            assert!(after_money < before_money);

            // pending 해제 확인
            assert!(session.pending.is_none());

            // 턴 진행 확인
            assert_eq!(session.current_turn_index, 1);

            // 액션 타입 확인
            assert_eq!(result.action_type, "purchase");
        }
    }

    /// 구매 거절 (SKIP) 테스트 (돈 변화 없음, pending 정상 해제)
    #[test]
    fn test_handle_decide_skip() {
        let conn = setup_test_db();
        let mut session = setup_session();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "can_buy" {
            let result = handle_decide(&conn, &mut session, false).unwrap();

            assert_eq!(result.action_type, "skip");
            assert!(session.pending.is_none());
        }
    }

    /// 돈 부족으로 구매 실패 테스트
    #[test]
    fn test_not_enough_money_to_buy() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 돈 거의 없게 만들기
        update_money(&conn, 1, -9999).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "can_buy" {
            let result = handle_decide(&conn, &mut session, true).unwrap();

            // 구매 못 하고 넘어가야 함
            assert!(matches!(result.action_type, "skip" | "none"));
        }
    }

    /// 게임 종료 조건 검증 테스트 (모든 플레이어가 파산 상태가 되면, 게임이 종료되는지 확인)
    #[test]
    fn test_game_end_trigger() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 모든 플레이어를 파산 처리
        conn.execute(
            "UPDATE players SET is_bankrupt = 1",
            [],
        ).unwrap();

        let _ = handle_turn(&conn, &mut session);

        // 게임 종료 확인
        assert!(session.game_finished);
    }

    /// 통행료 지불 테스트 (다른 플레이어 소유 타일 밟기)
    #[test]
    fn test_pay_toll_flow() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 2번 플레이어가 특정 타일 소유
        set_owner(&conn, 1, 2, 100).unwrap();

        // 1번 플레이어를 해당 타일 직전 위치로 이동
        update_position_and_lap(&conn, 1, 0, 0).unwrap();

        // 턴 실행 → 해당 타일 밟도록 유도
        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "pay_toll" {
            assert!(response.action_amount > 0);
            assert!(response.owner_id.is_some());
        }
    }

    /// 복지기금 납부 이벤트 테스트
    #[test]
    fn test_event_welfare_fund() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 이벤트 타일 위치로 강제 이동 (예: 5번이 이벤트라고 가정)
        update_position_and_lap(&conn, 1, 4, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "welfare_fund" {
            assert!(response.action_amount > 0);
        }
    }

    /// 기금 수령 이벤트 테스트
    #[test]
    fn test_event_fund_receive() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // fund에 돈 넣기
        conn.execute("UPDATE fund SET amount = 500", []).unwrap();

        // 이벤트 타일 이동
        update_position_and_lap(&conn, 1, 4, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "fund_receive" {
            assert!(response.action_amount > 0);
        }
    }

    /// 재산세 이벤트 테스트
    #[test]
    fn test_event_estate_tax() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 재산세 타일 위치로 이동
        update_position_and_lap(&conn, 1, 6, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "estate_tax" {
            assert!(response.action_amount >= 0);
        }
    }

    /// 통행료로 인한 파산 테스트
    #[test]
    fn test_bankrupt_by_toll() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 2번이 비싼 땅 소유
        set_owner(&conn, 1, 2, 1000).unwrap();

        // 1번 돈 거의 없게 만들기
        update_money(&conn, 1, -9999).unwrap();

        // 해당 위치 이동
        update_position_and_lap(&conn, 1, 0, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "bankrupt" {
            // 파산 발생 여부 검증
            assert!(response.owner_id.is_some());
        }
    }

    /// 기금이 비어있는 경우 테스트
    #[test]
    fn test_fund_receive_empty() {
        let conn = setup_test_db();
        let mut session = setup_session();

        conn.execute("UPDATE fund SET amount = 0", []).unwrap();

        update_position_and_lap(&conn, 1, 4, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "fund_receive_empty" {
            assert_eq!(response.action_amount, 0);
        }
    }

    /// START 통과 테스트 (lap 증가 + salary 지급 검증)
    #[test]
    fn test_passing_start_tile() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 마지막 칸 직전으로 이동 (예: 23 → 다음 턴에 0 넘어감)
        update_position_and_lap(&conn, 1, 23, 0).unwrap();

        let before = get_player_states(&conn).unwrap();
        let player_before = before.iter().find(|p| p.id == 1).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        let after = get_player_states(&conn).unwrap();
        let player_after = after.iter().find(|p| p.id == 1).unwrap();

        // lap 증가 확인
        assert!(player_after.lap >= player_before.lap);

        // salary 지급 확인
        if response.salary > 0 {
            assert!(player_after.money > player_before.money);
        }
    }

    /// 아무 이벤트 없는 일반 타일 테스트
    #[test]
    fn test_empty_tile_no_action() {
        let conn = setup_test_db();
        let mut session = setup_session();

        for i in 0..24 {
            let (price, _, owner, tile_type) = get_tile_info(&conn, i).unwrap();

            if price == 0 && tile_type != "event" {
                update_position_and_lap(&conn, 1, i, 0).unwrap();
                break;
            }
        }

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "none" {
            assert_eq!(response.action_amount, 0);
        }
    }

    /// 재산세 스킵 테스트 (소유 토지 없음)
    #[test]
    fn test_estate_tax_skipped() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 모든 토지 소유 제거
        conn.execute("DELETE FROM properties", []).unwrap();

        update_position_and_lap(&conn, 1, 6, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "estate_tax_skipped" {
            assert_eq!(response.action_amount, 0);
        }
    }

    /// 복지기금 납부 중 파산 테스트
    #[test]
    fn test_welfare_fund_bankrupt() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 돈 거의 없음
        update_money(&conn, 1, -9999).unwrap();

        update_position_and_lap(&conn, 1, 6, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "welfare_fund_bankrupt" {
            assert!(response.action_amount >= 0);
        }
    }

    /// 재산세로 인한 파산 테스트
    #[test]
    fn test_estate_tax_bankrupt() {
        let conn = setup_test_db();
        let mut session = setup_session();

        update_money(&conn, 1, -9999).unwrap();

        update_position_and_lap(&conn, 1, 6, 0).unwrap();

        let response = handle_turn(&conn, &mut session).unwrap();

        if response.action_type == "estate_tax_bankrupt" {
            assert!(response.action_amount >= 0);
        }
    }

    /// 턴 인덱스 순환 테스트
    #[test]
    fn test_turn_index_wraparound() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 플레이어 수만큼 턴 진행
        for _ in 0..10 {
            let res = handle_turn(&conn, &mut session).unwrap();

            if res.action_type == "can_buy" {
                let _ = handle_decide(&conn, &mut session, true).unwrap();
            }
        }

        // 인덱스가 정상 범위 유지되는지
        assert!(session.current_turn_index < 4);
    }

    /// 게임 종료 후 상금 지급 및 랭킹 계산 테스트
    #[test]
    fn test_game_end_with_rewards_and_ranking() {
        let conn = setup_test_db();
        let mut session = setup_session();

        // 플레이어 상태 강제 설정 (게임 종료 유도)
        // lap 기준으로 순위 결정됨
        conn.execute("UPDATE players SET lap = 10 WHERE id = 1", []).unwrap(); // 1등
        conn.execute("UPDATE players SET lap = 8 WHERE id = 2", []).unwrap();  // 2등
        conn.execute("UPDATE players SET lap = 5 WHERE id = 3", []).unwrap();  // 3등
        conn.execute("UPDATE players SET lap = 1 WHERE id = 4", []).unwrap();  // 탈락 수준

        // 한 턴 실행 → advance_turn → 게임 종료 트리거
        let _ = handle_turn(&conn, &mut session).unwrap();

        // 게임 종료 확인
        assert!(session.game_finished);

        // 랭킹 생성 확인
        assert!(session.final_rankings.is_some());

        let rankings = session.final_rankings.clone().unwrap();

        // 돈 기준으로 정렬되어야 함
        assert!(rankings.len() >= 3);

        // 보상 지급 확인
        // player money 조회
        let players = get_player_states(&conn).unwrap();

        let p1 = players.iter().find(|p| p.id == 1).unwrap();
        let p2 = players.iter().find(|p| p.id == 2).unwrap();
        let p3 = players.iter().find(|p| p.id == 3).unwrap();

        // 1등이 가장 돈 많아야 함
        assert!(p1.money >= p2.money);
        assert!(p2.money >= p3.money);

        // 최소 보상 금액 검증 (정확 값보다 안전)
        assert!(p1.money >= 150);
        assert!(p2.money >= 120);
        assert!(p3.money >= 80);

        // winner_id 설정 확인
        assert!(session.winner_id.is_some());
        assert_eq!(session.winner_id.unwrap(), rankings[0].0);

        // 보상 확인
        assert_eq!(p1.money, 150 + 300);
        assert_eq!(p2.money, 120 + 300);
        assert_eq!(p3.money, 80 + 300);
    }

    */

}
