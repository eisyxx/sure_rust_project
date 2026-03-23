use rusqlite::Connection;
use project::repository::init::init_db::init_db;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    init_db(&conn).unwrap(); //매 테스트가 새 DB 생성 후 시작되도록

    conn
}

use project::service::turn_service::{process_turn, TurnInput, TurnAction};

#[test]
fn test_turn_service_event_a_action() {
    let conn = setup_test_db();

    conn.execute("UPDATE players SET position = 5, lap = 0, money = 100 WHERE id = 1", []).unwrap();

    let input = TurnInput {
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
    };

    let result = process_turn(input, &conn);

    match result.action {
        TurnAction::EventWelfareFund { amount } => {
            assert_eq!(amount, 10);
        }
        _ => panic!("Expected EventWelfareFund"),
    }
}