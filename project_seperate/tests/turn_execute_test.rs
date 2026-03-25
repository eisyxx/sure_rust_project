/*
Purchase → 돈 감소 + owner 설정
PayToll → 돈 이동
Bankrupt → 파산 처리 + 토지 초기화
이벤트 → 각 케이스별 DB 반영
*/

#[cfg(test)]
mod tests {
    use project::service::turn_execute_service::apply_turn_result;
    use project::service::turn_service::{TurnAction, TurnResult};

    use rusqlite::Connection;

    #[test]
    fn test_apply_purchase() {
        let conn = Connection::open_in_memory().unwrap();

        let result = TurnResult {
            dice: 1,
            new_position: 1,
            new_lap: 0,
            salary: 0,
            action: TurnAction::Purchase { price: 50 },
        };

        let _ = apply_turn_result(&conn, 1, &result);

        // TODO: DB 검증 추가
    }
}