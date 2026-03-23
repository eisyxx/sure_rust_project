use rusqlite::Connection;
use project::repository::init::init_db::init_db;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    init_db(&conn).unwrap(); //매 테스트가 새 DB 생성 후 시작되도록

    conn
}

use project::service::turn_service::{process_turn, TurnInput};
use project::service::turn_execute_service::apply_turn_result;
use project::repository::player_repo::get_player_money;

#[test]
fn test_full_flow_event_c() {
    let conn = setup_test_db();

    // 플레이어 생성
    conn.execute(
        "UPDATE players SET position = 17, lap = 0, money = 100 WHERE id = 1",
        [],
    ).unwrap();

    // 기금 50 설정
    conn.execute("UPDATE fund SET amount = 50", []).unwrap();

    let input = TurnInput {
        player_id: 1,
        position: 17,
        lap: 0,
        money: 100,
        total_tiles: 24,
        tile_price: 0,
        tile_toll: 0,
        owner: None,
        will_buy: false,
        tile_type: "event".to_string(),
    };

    let result = process_turn(input, &conn);

    apply_turn_result(&conn, 1, &result).unwrap();

    let money = get_player_money(&conn, 1).unwrap();

    // 💥 핵심 검증
    assert_eq!(money, 150);
}

#[test]
fn test_full_flow_event_a_fund_accumulate() {
    let conn = setup_test_db();

    conn.execute(
        "UPDATE players SET position = 5, lap = 0, money = 100 WHERE id = 1",
        [],
    ).unwrap();

    let result = process_turn(
        TurnInput {
            player_id: 1,
            position: 5,
            lap: 0,
            money: 100,
            total_tiles: 24,
            tile_price: 0,
            tile_toll: 0,
            owner: None,
            will_buy: false,
            tile_type: "event".to_string(),
        },
        &conn,
    );

    apply_turn_result(&conn, 1, &result).unwrap();

    let fund: i32 = conn
        .query_row("SELECT amount FROM fund", [], |row| row.get(0))
        .unwrap();

    assert_eq!(fund, 10);
}