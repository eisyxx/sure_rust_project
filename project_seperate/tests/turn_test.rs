use mockall::mock;
use mockall::predicate::eq;
use project::service::event_service::EventResult;
use project::service::turn_service::{
    build_turn_result,
    build_turn_result_with_repo,
    roll_and_move,
    MoveStep,
    TurnAction,
    TurnEventRepository,
};
use rusqlite::Connection;

mock! {
    EventRepo {}

    impl TurnEventRepository for EventRepo {
        fn handle_event(&self, player_id: i32, tile_id: i32) -> EventResult;
    }
}

fn sample_step(position: i32) -> MoveStep {
    MoveStep {
        dice: 3,
        new_position: position,
        new_lap: 1,
        salary: 20,
    }
}

#[test]
/// 주사위/이동/월급 계산 함수가 정상 동작하는지 기본 검증
fn test_roll_and_move_runs() {
    let step = roll_and_move(0, 0, 20);
    assert!((1..=6).contains(&step.dice));
    assert!(step.new_position >= 0);
}

#[test]
/// 이벤트 결과 WelfareFund가 TurnAction::EventWelfareFund로 매핑되는지 검증
fn test_event_welfare_fund_mapping() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event()
        .with(eq(1), eq(7))
        .times(1)
        .returning(|_, _| EventResult::WelfareFund { amount: 50 });

    let result = build_turn_result_with_repo(&repo, sample_step(7), 1, 100, 60, 10, None, true, "event");
    assert_eq!(result.action, TurnAction::EventWelfareFund { amount: 50 });
}

#[test]
/// 이벤트 나머지 케이스들이 각 TurnAction으로 정확히 매핑되는지 검증
fn test_event_other_mappings() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event()
        .times(7)
        .returning_st(move |_, tile_id| match tile_id {
            1 => EventResult::WelfareFundBankrupt { paid: 11 },
            2 => EventResult::EstateTax { amount: 12 },
            3 => EventResult::EstateTaxBankrupt { paid: 13 },
            4 => EventResult::EstateTaxSkipped,
            5 => EventResult::FundReceive { amount: 14 },
            6 => EventResult::FundReceiveEmpty,
            _ => EventResult::None,
        });

    let r1 = build_turn_result_with_repo(&repo, sample_step(1), 1, 100, 60, 10, None, true, "event");
    let r2 = build_turn_result_with_repo(&repo, sample_step(2), 1, 100, 60, 10, None, true, "event");
    let r3 = build_turn_result_with_repo(&repo, sample_step(3), 1, 100, 60, 10, None, true, "event");
    let r4 = build_turn_result_with_repo(&repo, sample_step(4), 1, 100, 60, 10, None, true, "event");
    let r5 = build_turn_result_with_repo(&repo, sample_step(5), 1, 100, 60, 10, None, true, "event");
    let r6 = build_turn_result_with_repo(&repo, sample_step(6), 1, 100, 60, 10, None, true, "event");
    let r7 = build_turn_result_with_repo(&repo, sample_step(99), 1, 100, 60, 10, None, true, "event");

    assert_eq!(r1.action, TurnAction::EventWelfareFundBankrupt { paid: 11 });
    assert_eq!(r2.action, TurnAction::EstateTax { amount: 12 });
    assert_eq!(r3.action, TurnAction::EstateTaxBankrupt { paid: 13 });
    assert_eq!(r4.action, TurnAction::EstateTaxSkipped);
    assert_eq!(r5.action, TurnAction::EventFundReceive { amount: 14 });
    assert_eq!(r6.action, TurnAction::FundReceiveEmpty);
    assert_eq!(r7.action, TurnAction::None);
}

#[test]
/// 일반 타일에서 구매 가능 조건일 때 Purchase 액션이 선택되는지 검증
fn test_non_event_purchase() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event().never();

    let result = build_turn_result_with_repo(&repo, sample_step(2), 1, 100, 50, 10, None, true, "land");
    assert_eq!(result.action, TurnAction::Purchase { price: 50 });
}

#[test]
/// 일반 타일에서 소유자 존재 + 통행료 지불 가능일 때 PayToll 액션이 선택되는지 검증
fn test_non_event_pay_toll() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event().never();

    let result = build_turn_result_with_repo(&repo, sample_step(2), 1, 100, 50, 30, Some(2), true, "land");
    assert_eq!(result.action, TurnAction::PayToll { owner_id: 2, amount: 30 });
}

#[test]
/// 일반 타일에서 소유자 존재 + 통행료 지불 불가일 때 Bankrupt 액션이 선택되는지 검증
fn test_non_event_bankrupt() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event().never();

    let result = build_turn_result_with_repo(&repo, sample_step(2), 1, 5, 50, 30, Some(2), true, "land");
    assert_eq!(result.action, TurnAction::Bankrupt { owner_id: 2, paid: 5 });
}

#[test]
/// 일반 타일에서 구매 의사 있음 + 잔액 부족이면 None으로 매핑되는지 검증
fn test_non_event_not_enough_money_maps_none() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event().never();

    let result = build_turn_result_with_repo(&repo, sample_step(2), 1, 20, 50, 10, None, true, "land");
    assert_eq!(result.action, TurnAction::None);
}

#[test]
/// 일반 타일에서 구매 의사 없음이면 None으로 매핑되는지 검증
fn test_non_event_skip_maps_none() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event().never();

    let result = build_turn_result_with_repo(&repo, sample_step(2), 1, 200, 50, 10, None, false, "land");
    assert_eq!(result.action, TurnAction::None);
}

#[test]
/// 시작 타일(start)은 항상 None 액션으로 처리되는지 검증
fn test_start_tile_maps_none() {
    let mut repo = MockEventRepo::new();
    repo.expect_handle_event().never();

    let result = build_turn_result_with_repo(&repo, sample_step(2), 1, 200, 50, 10, None, true, "start");
    assert_eq!(result.action, TurnAction::None);
}

#[test]
/// Connection wrapper 경로를 통해 event 타일 처리 결과가 정상 매핑되는지 검증
fn test_build_turn_result_with_conn_wrapper_event_path() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute("CREATE TABLE event_tiles (tile_id INTEGER, event_type TEXT, amount INTEGER)", []).unwrap();
    conn.execute("CREATE TABLE players (id INTEGER, money INTEGER)", []).unwrap();
    conn.execute("CREATE TABLE properties (tile_id INTEGER, owner_id INTEGER, price INTEGER)", []).unwrap();
    conn.execute("CREATE TABLE fund (amount INTEGER)", []).unwrap();

    conn.execute("INSERT INTO event_tiles VALUES (7, 'fund_add', 30)", []).unwrap();
    conn.execute("INSERT INTO players VALUES (1, 100)", []).unwrap();
    conn.execute("INSERT INTO fund VALUES (0)", []).unwrap();

    let result = build_turn_result(
        &conn,
        sample_step(7),
        1,
        100,
        50,
        10,
        None,
        true,
        "event",
    );

    assert_eq!(result.action, TurnAction::EventWelfareFund { amount: 30 });
}
