use rusqlite::Connection;

use project::service::turn_service::{process_turn, TurnInput};
use project::service::turn_execute_service::apply_turn_result;

#[test]
fn test_full_flow_event_c() {
    let conn = Connection::open_in_memory().unwrap();

    // ---------------------------
    // 1. 테이블 생성
    // ---------------------------
    conn.execute(
        "CREATE TABLE players (
            id INTEGER PRIMARY KEY,
            money INTEGER,
            position INTEGER,
            lap INTEGER
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE tiles (
            id INTEGER,
            name TEXT,
            type TEXT,
            price INTEGER,
            toll INTEGER
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE event_tiles (
            tile_id INTEGER,
            event_type TEXT,
            amount INTEGER
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE fund (
            amount INTEGER
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE properties (
            tile_id INTEGER,
            owner_id INTEGER,
            price INTEGER
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            player_id INTEGER,
            type TEXT,
            amount INTEGER,
            description TEXT
        )",
        [],
    ).unwrap();

    // ---------------------------
    // 2. 더미 데이터 삽입
    // ---------------------------

    // 플레이어
    conn.execute(
        "INSERT INTO players VALUES (1, 50, 0, 0)",
        [],
    ).unwrap();

    // 이벤트 타일 (기금 수령)
    conn.execute(
        "INSERT INTO tiles VALUES (1, '기금수령', 'event', 0, 0)",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO event_tiles VALUES (1, 'fund_take', 0)",
        [],
    ).unwrap();

    // 현재 기금
    conn.execute(
        "INSERT INTO fund VALUES (100)",
        [],
    ).unwrap();

    // ---------------------------
    // 3. TurnInput 구성
    // ---------------------------
    let input = TurnInput {
        player_id: 1,
        position: 0,
        lap: 0,
        money: 50,
        total_tiles: 10,

        tile_price: 0,
        tile_toll: 0,
        owner: None,

        will_buy: false,
        tile_type: "event".to_string(),
    };

    // ---------------------------
    // 4. 턴 실행
    // ---------------------------
    let result = process_turn(input, &conn);

    println!("turn result: {:?}", result.action);

    // ---------------------------
    // 5. DB 반영
    // ---------------------------
    apply_turn_result(&conn, 1, &result).unwrap();

    // ---------------------------
    // 6. 결과 검증
    // ---------------------------
    let money: i32 = conn.query_row(
        "SELECT money FROM players WHERE id = 1",
        [],
        |row| row.get(0),
    ).unwrap();

    println!("final money: {}", money);

    // 기존 50 + 기금 100 = 150 기대
    assert_eq!(money, 150);
}