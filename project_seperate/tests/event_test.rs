use rusqlite::Connection;
use project::repository::init::init_db::init_db;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    init_db(&conn).unwrap(); //매 테스트가 새 DB 생성 후 시작되도록

    conn
}
use project::service::event_service::{handle_event, EventResult};

#[test]
fn test_event_a_welfare_fund() {
    let conn = setup_test_db();

    // 플레이어 돈 설정
    conn.execute("UPDATE players SET money = 100 WHERE id = 1", []).unwrap();

    // tile_id = 6 → 사회복지기금
    let result = handle_event(&conn, 1, 6);

    match result {
        EventResult::WelfareFund { amount } => {
            assert_eq!(amount, 10);
        }
        _ => panic!("Expected WelfareFund"),
    }
}

#[test]
fn test_event_a_bankrupt() {
    let conn = setup_test_db();

    conn.execute("UPDATE players SET money = 5 WHERE id = 1", []).unwrap();

    let result = handle_event(&conn, 1, 6);

    match result {
        EventResult::WelfareFundBankrupt { paid } => {
            assert_eq!(paid, 5);
        }
        _ => panic!("Expected WelfareFundBankrupt"),
    }
}

#[test]
fn test_event_c_fund_receive() {
    let conn = setup_test_db();

    conn.execute("UPDATE players SET money = 100 WHERE id = 1", []).unwrap();

    // 기금 50 쌓기
    conn.execute("UPDATE fund SET amount = 50", []).unwrap();

    // tile_id = 18 → 기금 수령
    let result = handle_event(&conn, 1, 18);

    match result {
        EventResult::FundReceive { amount } => {
            assert_eq!(amount, 50);
        }
        _ => panic!("Expected FundReceive"),
    }
}

#[test]
fn test_event_b_estate_tax() {
    let conn = setup_test_db();

    conn.execute("UPDATE players SET money = 200 WHERE id = 1", []).unwrap();

    // property 총합 100 이상 되게
    conn.execute("UPDATE properties SET owner_id = 1 WHERE tile_id = 1", []).unwrap();

    let result = handle_event(&conn, 1, 12); // 종부세

    match result {
        EventResult::EstateTax { amount } => {
            assert_eq!(amount, 30);
        }
        _ => panic!("Expected EstateTax"),
    }
}