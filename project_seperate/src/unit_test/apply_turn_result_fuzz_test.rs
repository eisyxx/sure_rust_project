/*
-fuzz testing-
랜덤한 게임 상황(action)에서 DB 상태가 깨지지 않는지 검증
*/

#[cfg(test)]
mod fuzz_apply_turn_result {
    use proptest::prelude::*;
    use rusqlite::Connection;

    use crate::service::turn_execute_service::apply_turn_result;
    use crate::service::turn_service::{TurnResult, TurnAction};

    // 간단한 DB 초기화
    fn setup_db(conn: &Connection, money: i32) {
        conn.execute(
            "CREATE TABLE players (
                id INTEGER PRIMARY KEY,
                money INTEGER,
                position INTEGER,
                lap INTEGER,
                is_bankrupt INTEGER
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

        conn.execute(
            "INSERT INTO players (id, money, position, lap, is_bankrupt)
             VALUES (1, ?, 0, 0, 0)",
            [money],
        ).unwrap();

        conn.execute(
            "INSERT INTO players (id, money, position, lap, is_bankrupt)
             VALUES (2, 10000, 0, 0, 0)",
            [],
        ).unwrap();
    }

    // 랜덤 TurnAction 생성
    fn arb_action() -> impl Strategy<Value = TurnAction> {
        prop_oneof![
            (1..3, 0..5000).prop_map(|(owner, amount)| TurnAction::PayToll {
                owner_id: owner,
                amount
            }),
            (1..3, 0..5000).prop_map(|(owner, paid)| TurnAction::Bankrupt {
                owner_id: owner,
                paid
            }),
            (0..5000).prop_map(|amount| TurnAction::EventWelfareFund { amount }),
            (0..5000).prop_map(|paid| TurnAction::EventWelfareFundBankrupt { paid }),
            (0..5000).prop_map(|amount| TurnAction::EventFundReceive { amount }),
            (0..5000).prop_map(|amount| TurnAction::EstateTax { amount }),
            (0..5000).prop_map(|paid| TurnAction::EstateTaxBankrupt { paid }),
            Just(TurnAction::FundReceiveEmpty),
            Just(TurnAction::EstateTaxSkipped),
            Just(TurnAction::None),
        ]
    }

    // 랜덤한 게임 상황(action)에서 DB 상태가 깨지지 않는지 검증
    proptest! {
        #[test]
        fn fuzz_apply_turn_result(
            initial_money in -10000i32..10000,
            action in arb_action(),
            dice in 1i32..=6,
        ) {
            let conn = Connection::open_in_memory().unwrap();
            setup_db(&conn, initial_money);

            let result = TurnResult {
                dice,
                new_position: 5,
                new_lap: 1,
                salary: 0,
                action,
            };

            let _ = apply_turn_result(&conn, 1, &result);

            // DB 상태 확인
            let (money, is_bankrupt): (i32, i32) =
                conn.query_row(
                    "SELECT money, is_bankrupt FROM players WHERE id=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                ).unwrap();

            // ━━━━━━━━━━━━━━━━━━━━━━━
            // invariant
            // ━━━━━━━━━━━━━━━━━━━━━━━

            // 1. 돈 overflow 방지
            prop_assert!(money > -1_000_000);
            prop_assert!(money < 1_000_000);

            // 2. 파산 상태 consistency
            if is_bankrupt == 1 {
                prop_assert!(money <= 0);
            }

            // 3. 돈이 양수면 파산 아님
            if money > 0 {
                prop_assert!(is_bankrupt == 0);
            }

            // 4. position 항상 정상 범위 (네 move_player 기준)
            let pos: i32 = conn.query_row(
                "SELECT position FROM players WHERE id=1",
                [],
                |r| r.get(0),
            ).unwrap();

            prop_assert!(pos >= 0);
            prop_assert!(pos < 24);

            // 5. lap 감소 금지
            let lap: i32 = conn.query_row(
                "SELECT lap FROM players WHERE id=1",
                [],
                |r| r.get(0),
            ).unwrap();

            prop_assert!(lap >= 0);
        }
    }
}