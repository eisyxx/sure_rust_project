/*
**DB 연결 함수를 제외하고 build_turn_result()함수만 테스트하는 경우

일반 타일 → Purchase / PayToll / Bankrupt
이벤트 타일 → 각 이벤트 결과 매핑
*/

#[cfg(test)]
mod tests {
    use project::service::turn_service::{build_turn_result, MoveStep, TurnAction, TurnInput, TurnResult};
    use rusqlite::Connection;

    #[test]
    fn test_build_turn_purchase() {
        let conn = Connection::open_in_memory().unwrap();

        let move_step = MoveStep {
            dice: 3,
            new_position: 1,
            new_lap: 0,
            salary: 0,
        };

        let result = build_turn_result(
            &conn,
            move_step,
            1,
            100,
            50,
            10,
            None,
            true,
            "land",
        );

        match result.action {
            TurnAction::Purchase { price } => assert_eq!(price, 50),
            _ => panic!("Expected Purchase"),
        }
    }
}


/*
**process_turn()은 DB 포함 테스트
일반 타일 → Purchase / PayToll / Bankrupt
- 이벤트 타일 → 이벤트 결과 반영


#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // tile 테이블
        conn.execute(
            "CREATE TABLE tiles (
                id INTEGER,
                price INTEGER,
                toll INTEGER,
                type TEXT
            )",
            [],
        ).unwrap();

        // property 테이블
        conn.execute(
            "CREATE TABLE properties (
                tile_id INTEGER,
                owner_id INTEGER
            )",
            [],
        ).unwrap();

        // event 테이블
        conn.execute(
            "CREATE TABLE events (
                tile_id INTEGER,
                type TEXT,
                amount INTEGER
            )",
            [],
        ).unwrap();

        // player 테이블
        conn.execute(
            "CREATE TABLE players (
                id INTEGER,
                money INTEGER
            )",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_process_turn_purchase() {
        let conn = setup_db();

        // 타일: 구매 가능한 땅
        conn.execute(
            "INSERT INTO tiles VALUES (1, 50, 10, 'land')",
            [],
        ).unwrap();

        let input = TurnInput {
            player_id: 1,
            position: 0,
            lap: 0,
            money: 100,
            total_tiles: 10,
            tile_price: 0,
            tile_toll: 0,
            owner: None,
            will_buy: true,
            tile_type: "land".to_string(),
        };

        let result = process_turn(input, &conn);

        match result.action {
            TurnAction::Purchase { price } => assert_eq!(price, 50),
            _ => panic!("Expected Purchase"),
        }
    }

    #[test]
    fn test_process_turn_event() {
        let conn = setup_db();

        // 이벤트 타일
        conn.execute(
            "INSERT INTO tiles VALUES (1, 0, 0, 'event')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO events VALUES (1, 'fund_add', 30)",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO players VALUES (1, 100)",
            [],
        ).unwrap();

        let input = TurnInput {
            player_id: 1,
            position: 0,
            lap: 0,
            money: 100,
            total_tiles: 10,
            tile_price: 0,
            tile_toll: 0,
            owner: None,
            will_buy: false,
            tile_type: "event".to_string(),
        };

        let result = process_turn(input, &conn);

        assert!(matches!(result.action, TurnAction::EventWelfareFund { .. }));
    }
}

    */