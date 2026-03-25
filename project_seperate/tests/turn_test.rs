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
