use mockall::mock;
use mockall::predicate::eq;
use project::service::turn_execute_service::{
    apply_turn_result,
    apply_turn_result_with_repo,
    TurnExecuteRepository,
};
use project::service::turn_service::{TurnAction, TurnResult};
use rusqlite::Connection;

fn any_err() -> rusqlite::Error {
    rusqlite::Error::InvalidQuery
}

mock! {
    ExecRepo {}

    impl TurnExecuteRepository for ExecRepo {
        fn update_position_and_lap(&self, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()>;
        fn update_money(&self, player_id: i32, delta: i32) -> rusqlite::Result<()>;
        fn bankrupt(&self, player_id: i32) -> rusqlite::Result<()>;
        fn set_owner(&self, tile_id: i32, owner_id: i32, price: i32) -> rusqlite::Result<()>;
        fn reset_owner_for_player(&self, player_id: i32) -> rusqlite::Result<()>;
        fn record_transaction(&self, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()>;
        fn add_fund(&self, amount: i32) -> rusqlite::Result<()>;
        fn reset_fund(&self) -> rusqlite::Result<()>;
    }
}

fn make_result(action: TurnAction, salary: i32) -> TurnResult {
    TurnResult {
        dice: 4,
        new_position: 10,
        new_lap: 2,
        salary,
        action,
    }
}

fn setup_db_for_wrapper() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute(
        "CREATE TABLE players (id INTEGER PRIMARY KEY, money INTEGER, position INTEGER, lap INTEGER, is_bankrupt INTEGER)",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            player_id INTEGER,
            type TEXT,
            amount INTEGER,
            target TEXT,
            balance_before INTEGER,
            balance_after INTEGER,
            created_at TEXT
        )",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE properties (tile_id INTEGER PRIMARY KEY, owner_id INTEGER, price INTEGER)",
        [],
    )
    .unwrap();
    conn.execute("CREATE TABLE fund (amount INTEGER)", []).unwrap();

    conn.execute("INSERT INTO players VALUES (1, 100, 0, 0, 0)", []).unwrap();
    conn.execute("INSERT INTO players VALUES (2, 100, 0, 0, 0)", []).unwrap();
    conn.execute("INSERT INTO properties VALUES (10, 1, 50)", []).unwrap();
    conn.execute("INSERT INTO properties VALUES (11, 1, 60)", []).unwrap();
    conn.execute("INSERT INTO fund VALUES (200)", []).unwrap();

    conn
}

#[test]
/// 월급/액션이 없는 기본 케이스에서 위치와 lap 업데이트만 수행되는지 검증
fn test_none_action_no_salary() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::None, 0);

    repo.expect_update_position_and_lap()
        .with(eq(1), eq(10), eq(2))
        .times(1)
        .returning(|_, _, _| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// 월급이 있는 케이스에서 입금 및 salary 거래 기록이 수행되는지 검증
fn test_salary_path() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::None, 20);

    repo.expect_update_position_and_lap()
        .with(eq(1), eq(10), eq(2))
        .times(1)
        .returning(|_, _, _| Ok(()));
    repo.expect_update_money()
        .with(eq(1), eq(20))
        .times(1)
        .returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("deposit"), eq(20), eq("salary"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// Purchase 액션에서 출금/거래기록/소유권 설정이 수행되는지 검증
fn test_purchase_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Purchase { price: 50 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-50)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(50), eq("tile10_purchase"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_set_owner().with(eq(10), eq(1), eq(50)).times(1).returning(|_, _, _| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// PayToll 액션에서 플레이어 출금 및 소유자 입금 흐름이 수행되는지 검증
fn test_pay_toll_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::PayToll { owner_id: 2, amount: 30 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-30)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(30), eq("toll_to_2"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(30)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(2), eq("deposit"), eq(30), eq("toll_from_1"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// Bankrupt 액션에서 잔액 정산/거래기록/소유권 초기화/파산 처리가 수행되는지 검증
fn test_bankrupt_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Bankrupt { owner_id: 2, paid: 40 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-40)).times(1).returning(|_, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(40)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(2), eq("deposit"), eq(40), eq("bankrupt_from_1"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(40), eq("bankrupt_to_2"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_reset_owner_for_player().with(eq(1)).times(1).returning(|_| Ok(()));
    repo.expect_bankrupt().with(eq(1)).times(1).returning(|_| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// EventWelfareFund 액션에서 기금 추가 및 거래기록이 수행되는지 검증
fn test_event_welfare_fund_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventWelfareFund { amount: 25 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-25)).times(1).returning(|_, _| Ok(()));
    repo.expect_add_fund().with(eq(25)).times(1).returning(|_| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(25), eq("welfare_fund"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// EventWelfareFundBankrupt 액션에서 기금 추가 후 파산 처리가 수행되는지 검증
fn test_event_welfare_fund_bankrupt_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventWelfareFundBankrupt { paid: 15 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_add_fund().with(eq(15)).times(1).returning(|_| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(15), eq("welfare_fund_bankrupt"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_reset_owner_for_player().with(eq(1)).times(1).returning(|_| Ok(()));
    repo.expect_bankrupt().with(eq(1)).times(1).returning(|_| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// FundReceiveEmpty 액션은 추가 DB 변경 없이 종료되는지 검증
fn test_fund_receive_empty_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::FundReceiveEmpty, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// EventFundReceive 액션에서 입금/거래기록/기금 초기화가 수행되는지 검증
fn test_event_fund_receive_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventFundReceive { amount: 70 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(70)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("deposit"), eq(70), eq("welfare_fund_receive"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_reset_fund().times(1).returning(|| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// EstateTaxSkipped 액션은 추가 DB 변경 없이 종료되는지 검증
fn test_estate_tax_skipped_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EstateTaxSkipped, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// EstateTax 액션에서 세금 출금 및 거래기록이 수행되는지 검증
fn test_estate_tax_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EstateTax { amount: 33 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-33)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(33), eq("estate_tax"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// EstateTaxBankrupt 액션에서 세금 출금 후 파산 처리가 수행되는지 검증
fn test_estate_tax_bankrupt_action() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EstateTaxBankrupt { paid: 22 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-22)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(22), eq("estate_tax_bankrupt"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_reset_owner_for_player().with(eq(1)).times(1).returning(|_| Ok(()));
    repo.expect_bankrupt().with(eq(1)).times(1).returning(|_| Ok(()));

    apply_turn_result_with_repo(&repo, 1, &result).unwrap();
}

#[test]
/// Connection wrapper 경로에서 TurnResult 반영(위치/lap)이 정상 동작하는지 검증
fn test_apply_turn_result_with_conn_wrapper() {
    let conn = setup_db_for_wrapper();

    let result = make_result(TurnAction::None, 0);
    apply_turn_result(&conn, 1, &result).unwrap();

    let (position, lap): (i32, i32) = conn
        .query_row("SELECT position, lap FROM players WHERE id = 1", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();

    assert_eq!(position, 10);
    assert_eq!(lap, 2);
}

#[test]
/// Connection wrapper 경로에서 Purchase 액션 반영(소유권 반영)이 동작하는지 검증
fn test_apply_turn_result_with_conn_purchase_path() {
    let conn = setup_db_for_wrapper();

    let result = make_result(TurnAction::Purchase { price: 40 }, 0);
    apply_turn_result(&conn, 1, &result).unwrap();

    let owner: Option<i32> = conn
        .query_row("SELECT owner_id FROM properties WHERE tile_id = 10", [], |row| row.get(0))
        .unwrap();
    assert_eq!(owner, Some(1));
}

#[test]
/// Connection wrapper 경로에서 Bankrupt 액션 반영(파산 플래그)이 동작하는지 검증
fn test_apply_turn_result_with_conn_bankrupt_path() {
    let conn = setup_db_for_wrapper();

    let result = make_result(TurnAction::Bankrupt { owner_id: 2, paid: 30 }, 0);
    apply_turn_result(&conn, 1, &result).unwrap();

    let is_bankrupt: i32 = conn
        .query_row("SELECT is_bankrupt FROM players WHERE id = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(is_bankrupt, 1);
}

#[test]
/// Connection wrapper 경로에서 EventWelfareFund 액션 반영(기금 증가)이 동작하는지 검증
fn test_apply_turn_result_with_conn_welfare_fund_path() {
    let conn = setup_db_for_wrapper();

    let result = make_result(TurnAction::EventWelfareFund { amount: 15 }, 0);
    apply_turn_result(&conn, 1, &result).unwrap();

    let fund: i32 = conn
        .query_row("SELECT amount FROM fund", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fund, 215);
}

#[test]
/// Connection wrapper 경로에서 EventFundReceive 액션 반영(기금 초기화)이 동작하는지 검증
fn test_apply_turn_result_with_conn_fund_receive_path() {
    let conn = setup_db_for_wrapper();

    let result = make_result(TurnAction::EventFundReceive { amount: 50 }, 0);
    apply_turn_result(&conn, 1, &result).unwrap();

    let fund: i32 = conn
        .query_row("SELECT amount FROM fund", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fund, 0);
}

#[test]
/// 위치/lap 업데이트 실패 시 즉시 에러를 전파하는지 검증
fn test_error_on_update_position_and_lap() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::None, 0);

    repo.expect_update_position_and_lap()
        .times(1)
        .returning(|_, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// salary 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_salary_deposit_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::None, 10);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(10)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("deposit"), eq(10), eq("salary"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// Purchase 액션의 set_owner 실패 시 에러를 전파하는지 검증
fn test_error_on_purchase_set_owner() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Purchase { price: 50 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-50)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(50), eq("tile10_purchase"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_set_owner().with(eq(10), eq(1), eq(50)).times(1).returning(|_, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// PayToll 액션에서 소유자 입금 실패 시 에러를 전파하는지 검증
fn test_error_on_pay_toll_second_transfer() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::PayToll { owner_id: 2, amount: 30 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-30)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(30), eq("toll_to_2"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(30)).times(1).returning(|_, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// Bankrupt 액션 마지막 파산 처리 실패 시 에러를 전파하는지 검증
fn test_error_on_bankrupt_finalize() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Bankrupt { owner_id: 2, paid: 40 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-40)).times(1).returning(|_, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(40)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(2), eq("deposit"), eq(40), eq("bankrupt_from_1"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(40), eq("bankrupt_to_2"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_reset_owner_for_player().with(eq(1)).times(1).returning(|_| Ok(()));
    repo.expect_bankrupt().with(eq(1)).times(1).returning(|_| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// EventWelfareFund 액션 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_welfare_fund_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventWelfareFund { amount: 25 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-25)).times(1).returning(|_, _| Ok(()));
    repo.expect_add_fund().with(eq(25)).times(1).returning(|_| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(25), eq("welfare_fund"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// EventFundReceive 액션의 reset_fund 실패 시 에러를 전파하는지 검증
fn test_error_on_fund_receive_reset() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventFundReceive { amount: 70 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(70)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("deposit"), eq(70), eq("welfare_fund_receive"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_reset_fund().times(1).returning(|| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// EstateTax 액션 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_estate_tax_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EstateTax { amount: 33 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-33)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(33), eq("estate_tax"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// Purchase 액션의 출금 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_purchase_withdraw_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Purchase { price: 50 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-50)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(50), eq("tile10_purchase"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// PayToll 액션의 플레이어 출금 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_pay_toll_from_player_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::PayToll { owner_id: 2, amount: 30 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-30)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(30), eq("toll_to_2"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// PayToll 액션의 소유자 입금 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_pay_toll_to_owner_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::PayToll { owner_id: 2, amount: 30 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-30)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(30), eq("toll_to_2"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(30)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(2), eq("deposit"), eq(30), eq("toll_from_1"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// Bankrupt 액션의 소유자 입금 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_bankrupt_deposit_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Bankrupt { owner_id: 2, paid: 40 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-40)).times(1).returning(|_, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(40)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(2), eq("deposit"), eq(40), eq("bankrupt_from_1"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// Bankrupt 액션의 파산자 출금 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_bankrupt_withdraw_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::Bankrupt { owner_id: 2, paid: 40 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-40)).times(1).returning(|_, _| Ok(()));
    repo.expect_update_money().with(eq(2), eq(40)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(2), eq("deposit"), eq(40), eq("bankrupt_from_1"))
        .times(1)
        .returning(|_, _, _, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(40), eq("bankrupt_to_2"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// EventWelfareFundBankrupt 액션 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_welfare_fund_bankrupt_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventWelfareFundBankrupt { paid: 15 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_add_fund().with(eq(15)).times(1).returning(|_| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(15), eq("welfare_fund_bankrupt"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// EventFundReceive 액션 입금 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_fund_receive_deposit_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EventFundReceive { amount: 70 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(70)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("deposit"), eq(70), eq("welfare_fund_receive"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}

#[test]
/// EstateTaxBankrupt 액션 거래기록 실패 시 에러를 전파하는지 검증
fn test_error_on_estate_tax_bankrupt_transaction() {
    let mut repo = MockExecRepo::new();
    let result = make_result(TurnAction::EstateTaxBankrupt { paid: 22 }, 0);

    repo.expect_update_position_and_lap().times(1).returning(|_, _, _| Ok(()));
    repo.expect_update_money().with(eq(1), eq(-22)).times(1).returning(|_, _| Ok(()));
    repo.expect_record_transaction()
        .with(eq(1), eq("withdraw"), eq(22), eq("estate_tax_bankrupt"))
        .times(1)
        .returning(|_, _, _, _| Err(any_err()));

    assert!(apply_turn_result_with_repo(&repo, 1, &result).is_err());
}