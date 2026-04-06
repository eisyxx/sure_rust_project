/// 자동으로 여러번 실행하며
/// 랜덤하게 돈, 위치, 주사위, 구매 선택을 테스트

#[cfg(test)]
mod fuzz_tests {
    use proptest::prelude::*;
    use rusqlite::Connection;

    use project::service::orchestrator::*;
    use project::service::traits::TurnServiceDeps;
    use project::service::event_service::EventResult;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Mock Dice (랜덤 제어)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    struct MockDeps {
        dice: i32,
    }

    impl TurnServiceDeps for MockDeps {
        fn roll_dice(&self) -> i32 {
            self.dice
        }

        fn handle_event(
            &self,
            _conn: &Connection,
            _player_id: i32,
            _tile_id: i32,
        ) -> EventResult {
            EventResult::None
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 랜덤 초기 상태 생성
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    fn seed_random_state(
        conn: &Connection,
        money: i32,
        position: i32,
    ) {
        let _ = conn.execute(
            "UPDATE players SET money=?, position=? WHERE id=1",
            [money, position],
        );
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 핵심 퍼징 테스트
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    proptest! {
        #[test]
        fn fuzz_turn_flow(
            dice in 1i32..=6,
            money in -1000i32..10000,
            position in 0i32..30,
            will_buy in any::<bool>(),
        ) {
/* 디버깅용
            println!(
                "[INPUT] dice={}, money={}, position={}, will_buy={}",
                dice, money, position, will_buy
            );
*/

            let conn = Connection::open_in_memory().unwrap();
            let mut session = init_session(&conn).unwrap();

            // 랜덤 상태 주입
            seed_random_state(&conn, money, position);


/* 디버깅용
            let (pos, money_db): (i32, i32) = conn.query_row(
                "SELECT position, money FROM players WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).unwrap();
            println!("[DB AFTER SEED] pos={}, money={}", pos, money_db);
*/

            session.current_turn_index = 0;

            let repo = TurnRepoImpl;
            let deps = MockDeps { dice };

            // ━━━━━━━━━━━━━━━━━━━━━━━
            // 1. turn 실행
            // ━━━━━━━━━━━━━━━━━━━━━━━
            let result = process_turn_with_repo(
                &repo,
                &deps,
                &conn,
                &mut session,
            );

            // 크래시 방지 (panic 나면 실패)
            prop_assert!(result.is_ok());

            let result = result.unwrap();

            // ━━━━━━━━━━━━━━━━━━━━━━━
            // 2. decide (조건부)
            // ━━━━━━━━━━━━━━━━━━━━━━━
            let final_result = if result.action_type == "can_buy" {
                let decide = process_decide(&conn, &mut session, will_buy);
                prop_assert!(decide.is_ok());
                decide.unwrap()
            } else {
                result
            };

/* 디버깅용
            println!(
                "[FINAL RESULT] action={}, pos={}, lap={}",
                final_result.action_type,
                final_result.new_position,
                final_result.new_lap
            ); 
*/

            // ━━━━━━━━━━━━━━━━━━━━━━━
            // Invariant 검증
            // ━━━━━━━━━━━━━━━━━━━━━━━

            // 1. 위치 정상 범위
            prop_assert!(final_result.new_position >= 0);
            prop_assert!(final_result.new_position < 24);

            // 2. lap 감소 금지
            prop_assert!(final_result.new_lap >= final_result.old_lap);

            // 3. salary 정상
            prop_assert!(
                final_result.salary == 0 ||
                final_result.salary == 20
            );

            // 4. action_type 유효성
            let valid_actions = [
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

            prop_assert!(valid_actions.contains(&final_result.action_type));

            // 5. DB 상태 검증
            let (pos, money, is_bankrupt): (i32, i32, i32) =
                conn.query_row(
                    "SELECT position, money, is_bankrupt FROM players WHERE id=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                ).unwrap();

            // 위치 일관성
            prop_assert_eq!(pos, final_result.new_position);

            // 돈 overflow 방지
            prop_assert!(money > -1_000_000);

            // 파산 상태 consistency
            if is_bankrupt == 1 {
                prop_assert!(money <= 0);
            }
        }

        /// API (turn, decide)를 랜덤한 순서로 호출
        #[test]
            fn fuzz_state_machine(
                actions in prop::collection::vec(0u8..3, 1..50), // 0: turn, 1: decide(true), 2: decide(false)
                dice in 1i32..=6,
            ) {
                let conn = Connection::open_in_memory().unwrap();
                let mut session = init_session(&conn).unwrap();

                let repo = TurnRepoImpl;
                let deps = MockDeps { dice };

                for action in actions {
                    match action {
                        // turn 호출
                        0 => {
                            let _ = process_turn_with_repo(&repo, &deps, &conn, &mut session);
                        }

                        // decide(true)
                        1 => {
                            let _ = process_decide(&conn, &mut session, true);
                        }

                        // decide(false)
                        2 => {
                            let _ = process_decide(&conn, &mut session, false);
                        }

                        _ => unreachable!(),
                    }


                    // 1. 게임 끝났으면 pending 없어야 함
                    if session.game_finished {
                        prop_assert!(session.pending.is_none());
                    }

                    // 2. pending 있으면 turn 호출하면 안 됨 (또는 안전해야 함)
                    if session.pending.is_some() {
                        // 여기서는 "죽지 않는 것"만 체크
                        prop_assert!(true);
                    }

                    // 3. current_turn_index 범위
                    prop_assert!(session.current_turn_index >= 0);

                    // 4. DB 상태 체크
                    let count: i32 = conn.query_row(
                        "SELECT COUNT(*) FROM players",
                        [],
                        |r| r.get(0),
                    ).unwrap();

                    prop_assert!(count > 0);
                }
            }
        }
}