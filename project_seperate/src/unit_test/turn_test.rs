#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::service::event_service::EventResult;
    use crate::service::turn_service::{
        build_turn_result_with_deps, roll_and_move_with_deps,
        MoveStep, TurnAction,
    };
    use crate::service::traits::TurnServiceDeps;

    // ── Mock ──────────────────────────────────────────────
    struct MockDeps {
        dice: i32,
        event_result: EventResult,
    }

    impl TurnServiceDeps for MockDeps {
        fn roll_dice(&self) -> i32 {
            self.dice
        }
        fn handle_event(&self, _conn: &Connection, _player_id: i32, _tile_id: i32) -> EventResult {
            // EventResult은 PartialEq가 있지만 Copy/Clone이 없어 직접 재구성
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

    fn dummy_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn make_move_step(dice: i32, pos: i32, lap: i32, salary: i32) -> MoveStep {
        MoveStep { dice, new_position: pos, new_lap: lap, salary }
    }

    // ── roll_and_move_with_deps ───────────────────────────
    #[test]
    fn roll_and_move_no_wrap() {
        let deps = MockDeps { dice: 3, event_result: EventResult::None };
        let result = roll_and_move_with_deps(&deps, 5, 0, 24);
        assert_eq!(result.dice, 3);
        assert_eq!(result.new_position, 8);
        assert_eq!(result.new_lap, 0);
        assert_eq!(result.salary, 0);
    }

    #[test]
    fn roll_and_move_wrap_around() {
        let deps = MockDeps { dice: 5, event_result: EventResult::None };
        let result = roll_and_move_with_deps(&deps, 22, 0, 24);
        assert_eq!(result.dice, 5);
        assert_eq!(result.new_position, 3);
        assert_eq!(result.new_lap, 1);
        assert_eq!(result.salary, 20);
    }

    // ── build_turn_result_with_deps: 이벤트 타일 ──────────
    #[test]
    fn event_welfare_fund() {
        let deps = MockDeps { dice: 0, event_result: EventResult::WelfareFund { amount: 50 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::EventWelfareFund { amount: 50 });
    }

    #[test]
    fn event_welfare_fund_bankrupt() {
        let deps = MockDeps { dice: 0, event_result: EventResult::WelfareFundBankrupt { paid: 30 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 30, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::EventWelfareFundBankrupt { paid: 30 });
    }

    #[test]
    fn event_estate_tax() {
        let deps = MockDeps { dice: 0, event_result: EventResult::EstateTax { amount: 40 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 200, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::EstateTax { amount: 40 });
    }

    #[test]
    fn event_estate_tax_bankrupt() {
        let deps = MockDeps { dice: 0, event_result: EventResult::EstateTaxBankrupt { paid: 20 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 20, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::EstateTaxBankrupt { paid: 20 });
    }

    #[test]
    fn event_estate_tax_skipped() {
        let deps = MockDeps { dice: 0, event_result: EventResult::EstateTaxSkipped };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 200, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::EstateTaxSkipped);
    }

    #[test]
    fn event_fund_receive() {
        let deps = MockDeps { dice: 0, event_result: EventResult::FundReceive { amount: 300 } };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::EventFundReceive { amount: 300 });
    }

    #[test]
    fn event_fund_receive_empty() {
        let deps = MockDeps { dice: 0, event_result: EventResult::FundReceiveEmpty };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::FundReceiveEmpty);
    }

    #[test]
    fn event_none() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 5, 0, 0),
            1, 100, 0, 0, None, false, "event",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    // ── build_turn_result_with_deps: 일반 타일 (buy_property) ──
    #[test]
    fn land_purchase() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 100, 50, 10, None, true, "land",
        );
        assert_eq!(result.action, TurnAction::Purchase { price: 50 });
    }

    #[test]
    fn land_pay_toll() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 100, 50, 10, Some(2), false, "land",
        );
        assert_eq!(result.action, TurnAction::PayToll { owner_id: 2, amount: 10 });
    }

    #[test]
    fn land_bankrupt() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 5, 50, 10, Some(2), false, "land",
        );
        assert_eq!(result.action, TurnAction::Bankrupt { owner_id: 2, paid: 5 });
    }

    #[test]
    fn land_not_enough_money() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 10, 50, 10, None, true, "land",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    #[test]
    fn land_skip_no_buy() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 1, 0, 0),
            1, 100, 50, 10, None, false, "land",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    #[test]
    fn start_tile_skip() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(3, 0, 0, 0),
            1, 100, 0, 0, None, false, "start",
        );
        assert_eq!(result.action, TurnAction::None);
    }

    // ── TurnResult 필드 검증 ──────────────────────────────
    #[test]
    fn turn_result_carries_move_step_fields() {
        let deps = MockDeps { dice: 0, event_result: EventResult::None };
        let result = build_turn_result_with_deps(
            &deps, &dummy_conn(), make_move_step(4, 7, 1, 20),
            1, 120, 0, 0, None, false, "start",
        );
        assert_eq!(result.dice, 4);
        assert_eq!(result.new_position, 7);
        assert_eq!(result.new_lap, 1);
        assert_eq!(result.salary, 20);
    }
}
