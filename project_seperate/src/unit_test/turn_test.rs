#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::service::event_service::EventResult;
    use crate::service::turn_service::{
        build_turn_result_with_deps, roll_and_move_with_deps,
        resolve_current_player_id_with_repo,
        build_landing_context_with_repo, get_active_game_players_with_repo,
        MoveStep, TurnAction,
    };
    use crate::service::traits::{TurnServiceDeps, PlayerStateRepo, TurnServiceQueryRepo};
    use crate::service::port_impl::PortImpl;

    use crate::repository::init::init_db;
    use crate::repository::player_repo::PlayerRow;
    use crate::repository::player_repo::PlayerState;

    // ── Mock ──────────────────────────────────────────────
    // 주사위 값과 이벤트 결과를 고정해서 반환하는 Mock
    struct MockDeps {
        dice: i32,
        event_result: EventResult,
    }

    impl TurnServiceDeps for MockDeps {
        fn roll_dice(&self) -> i32 {
            self.dice
        }
        fn handle_event(&self, _conn: &Connection, _player_id: i32, _tile_id: i32) -> EventResult {
            // EventResult는 Clone이 없어서 패턴 매칭으로 재생성
            match &self.event_result {
                EventResult::WelfareFund { amount } => EventResult::WelfareFund { amount: *amount },
                EventResult::WelfareFundBankrupt { paid } => EventResult::WelfareFundBankrupt { paid: *paid },
                EventResult::EstateTax { amount } => EventResult::EstateTax { amount: *amount },
                EventResult::EstateTaxBankrupt { paid } => EventResult::EstateTaxBankrupt { paid: *paid },
                EventResult::EstateTaxSkipped => EventResult::EstateTaxSkipped,
                EventResult::FundReceive { amount } => EventResult::FundReceive { amount: *amount },
                EventResult::FundReceiveEmpty => EventResult::FundReceiveEmpty,
                EventResult::None => EventResult::None,
            }
        }
    }

    // 인메모리 DB
    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    // 이동 결과 생성 헬퍼
    fn make_move_step(dice: i32, pos: i32, lap: i32, salary: i32) -> MoveStep {
        MoveStep { dice, new_position: pos, new_lap: lap, salary }
    }

    // ── roll_and_move_with_deps ───────────────────────────
    #[test]
    fn roll_and_move_no_wrap() {
        // 보드 끝을 넘지 않는 일반 이동:
        // - 위치만 증가
        // - lap 증가 없음
        // - salary 없음
        let deps = MockDeps { dice: 3, event_result: EventResult::None };
        let result = roll_and_move_with_deps(&deps, 5, 0, 24);
        assert_eq!(result.dice, 3);
        assert_eq!(result.new_position, 8);
        assert_eq!(result.new_lap, 0);
        assert_eq!(result.salary, 0);
    }

    #[test]
    fn roll_and_move_wrap_around() {
        // 보드 끝을 넘어가는 경우:
        // - 위치는 modulo 처리
        // - lap 증가
        // - salary 지급
        let deps = MockDeps { dice: 5, event_result: EventResult::None };
        let result = roll_and_move_with_deps(&deps, 22, 0, 24);
        assert_eq!(result.dice, 5);
        assert_eq!(result.new_position, 3);
        assert_eq!(result.new_lap, 1);
        assert_eq!(result.salary, 20);
    }

    // ── build_turn_result_with_deps: 이벤트 타일 ──────────
    #[test]
    // 복지기금 납부 이벤트
    fn event_welfare_fund() {
        let deps = MockDeps { dice: 0, event_result: EventResult::WelfareFund { amount: 50 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::EventWelfareFund { amount: 50 });
    }

    #[test]
    // 복지기금 내다가 파산
    fn event_welfare_fund_bankrupt() {
        let deps = MockDeps { dice: 0, event_result: EventResult::WelfareFundBankrupt { paid: 30 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 30, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::EventWelfareFundBankrupt { paid: 30 });
    }

    #[test]
    // 재산세 납부
    fn event_estate_tax() {
        let deps = MockDeps { dice: 0, event_result: EventResult::EstateTax { amount: 40 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 200, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::EstateTax { amount: 40 });
    }

    #[test]
    // 재산세 내다가 파산
    fn event_estate_tax_bankrupt() {
        let deps = MockDeps { dice: 0, event_result: EventResult::EstateTaxBankrupt { paid: 20 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 20, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::EstateTaxBankrupt { paid: 20 });
    }

    #[test]
    // 재산세 면제
    fn event_estate_tax_skipped() {
        let deps = MockDeps { dice: 0, event_result: EventResult::EstateTaxSkipped };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 200, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::EstateTaxSkipped);
    }

    #[test]
    // 복지기금 수령
    fn event_fund_receive() {
        let deps = MockDeps { dice: 0, event_result: EventResult::FundReceive { amount: 300 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::EventFundReceive { amount: 300 });
    }

    #[test]
    // 복지기금 없음
    fn event_fund_receive_empty() {
        let deps = MockDeps { dice: 0, event_result: EventResult::FundReceiveEmpty };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::FundReceiveEmpty);
    }

    #[test]
    // 이벤트 없음
    fn event_none() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, "event",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    // ── build_turn_result_with_deps: 일반 타일 (buy_property) ──
    #[test]
    // 주인 없는 땅 → 아무 액션 없음
    fn land_no_owner_skips() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 100, 50, 10, None, "land",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    #[test]
    // 주인이 있는 땅 → 통행료 지불
    fn land_pay_toll() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 100, 50, 10, Some(2), "land",
        );
        assert_eq!(result.action, TurnAction::PayToll { owner_id: 2, amount: 10 });
    }

    #[test]
    // 돈 부족 → 파산 처리
    fn land_bankrupt() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 5, 50, 10, Some(2), "land",
        );
        assert_eq!(result.action, TurnAction::Bankrupt { owner_id: 2, paid: 5 });
    }

    #[test]
    // 구매도 못하고 아무 행동도 안하는 케이스
    fn land_not_enough_money() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 10, 50, 10, None, "land",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    #[test]
    // 시작 타일 → 아무 액션 없음
    fn start_tile_skip() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 0, 0, 0),
            1, 100, 0, 0, None, "start",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    // ── TurnResult 필드 검증 ──────────────────────────────
    #[test]
    // MoveStep 값이 TurnResult로 그대로 전달되는지 확인
    fn turn_result_carries_move_step_fields() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(4, 7, 1, 20),
            1, 120, 0, 0, None, "start",
        );
        assert_eq!(result.dice, 4);
        assert_eq!(result.new_position, 7);
        assert_eq!(result.new_lap, 1);
        assert_eq!(result.salary, 20);
    }

    // ── resolve_current_player_id Mock ────────────────────
    // 플레이어 상태를 반환하는 Mock Repo
    struct MockPlayerStateRepo {
        players: Vec<PlayerState>,
    }

    impl PlayerStateRepo for MockPlayerStateRepo {
        fn get_player_states(&self, _conn: &Connection) -> rusqlite::Result<Vec<PlayerState>> {
            Ok(self.players.clone())
        }
    }

    fn make_player(id: i32, is_bankrupt: bool) -> PlayerState {
        PlayerState {
            id,
            name: format!("p{}", id),
            position: 0,
            money: 100,
            lap: 0,
            turn_order: id,
            is_bankrupt,
        }
    }

    // ── resolve_current_player_id_with_repo ───────────────
    #[test]
    // 모든 플레이어가 파산 → None 반환
    fn resolve_player_id_no_active_players() {
        let repo = MockPlayerStateRepo {
            players: vec![make_player(1, true), make_player(2, true)],
        };
        let result = resolve_current_player_id_with_repo(&repo, &dummy_conn(), 0).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    // turn index에 맞는 플레이어 반환
    fn resolve_player_id_returns_correct_player() {
        let repo = MockPlayerStateRepo {
            players: vec![make_player(1, false), make_player(2, false), make_player(3, false)],
        };
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 0).unwrap(), Some(1));
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 1).unwrap(), Some(2));
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 2).unwrap(), Some(3));
    }

    #[test]
    // 인덱스가 플레이어 수를 넘어가면 순환
    fn resolve_player_id_wraps_index() {
        let repo = MockPlayerStateRepo {
            players: vec![make_player(1, false), make_player(2, false)],
        };
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 2).unwrap(), Some(1));
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 3).unwrap(), Some(2));
    }

    #[test]
    // 파산한 플레이어는 건너뛰고 다음 플레이어 선택
    fn resolve_player_id_skips_bankrupt() {
        let repo = MockPlayerStateRepo {
            players: vec![make_player(1, true), make_player(2, false), make_player(3, false)],
        };
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 0).unwrap(), Some(2));
        assert_eq!(resolve_current_player_id_with_repo(&repo, &dummy_conn(), 1).unwrap(), Some(3));
    }

    // ── build_landing_context_with_repo ──────────────────
    struct MockTurnServiceQueryRepo {
        tile_info: (i32, i32, Option<i32>, String),
        owner: Option<i32>,
    }

    impl TurnServiceQueryRepo for MockTurnServiceQueryRepo {
        fn get_tile_info(&self, _conn: &Connection, _tile_id: i32) -> rusqlite::Result<(i32, i32, Option<i32>, String)> {
            Ok(self.tile_info.clone())
        }

        fn get_owner(&self, _conn: &Connection, _tile_id: i32) -> rusqlite::Result<Option<i32>> {
            Ok(self.owner)
        }

        fn get_all_players(&self, _conn: &Connection) -> rusqlite::Result<Vec<PlayerRow>> {
            Ok(vec![])
        }
    }

    #[test]
    fn landing_context_with_repo_builds_expected_values() {
        let repo = MockTurnServiceQueryRepo {
            tile_info: (120, 15, None, "land".to_string()),
            owner: Some(2),
        };

        let ctx = build_landing_context_with_repo(&repo, &dummy_conn(), 7, 80, 20);

        assert_eq!(ctx.tile_price, 120);
        assert_eq!(ctx.tile_toll, 15);
        assert_eq!(ctx.tile_owner, Some(2));
        assert_eq!(ctx.tile_type, "land");
        assert_eq!(ctx.money_after_salary, 100);
    }

    // ── get_active_game_players_with_repo ────────────────
    #[test]
    fn get_active_game_players_with_repo_filters_bankrupt_players() {
        let conn = dummy_conn();
        init_db::init_db(&conn).unwrap();

        // 한 명을 파산 처리해서 필터링 동작 확인
        conn.execute("UPDATE players SET is_bankrupt = 1 WHERE id = 1", []).unwrap();

        let players = get_active_game_players_with_repo(&PortImpl, &conn).unwrap();

        assert!(!players.is_empty());
        assert!(players.iter().all(|p| !p.is_bankrupt));
        assert!(players.iter().all(|p| p.id != 1));
    }

}
