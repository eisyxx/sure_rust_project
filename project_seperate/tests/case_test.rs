#[cfg(test)]

#[derive(Debug)]
    struct ExpectedTurnResult {
        player_id: i32,
        dice: i32,
        old_position: i32,
        new_position: i32,
        old_lap: i32,
        new_lap: i32,
        salary: i32,
        action_type: &'static str,
        action_amount: i32,
        owner_id: Option<i32>,
        current_player_id: Option<i32>,
        game_finished: bool,
        winner_id: Option<i32>,
    }
    
    fn assert_turn_result(result: &TurnResult, expected: ExpectedTurnResult) {
        assert_eq!(result.player_id, expected.player_id);
        assert_eq!(result.dice, expected.dice);
        assert_eq!(result.old_position, expected.old_position);
        assert_eq!(result.new_position, expected.new_position);
        assert_eq!(result.old_lap, expected.old_lap);
        assert_eq!(result.new_lap, expected.new_lap);
        assert_eq!(result.salary, expected.salary);
        assert_eq!(result.action_type, expected.action_type);
        assert_eq!(result.action_amount, expected.action_amount);
        assert_eq!(result.owner_id, expected.owner_id);
        assert_eq!(result.current_player_id, expected.current_player_id);
        assert_eq!(result.game_finished, expected.game_finished);
        assert_eq!(result.winner_id, expected.winner_id);
    }
    
mod integration_tests {
    use rusqlite::Connection;
    use project::service::orchestrator::*;
    use project::service::event_service::{handle_event, EventResult};
    use project::service::traits::TurnServiceDeps;

    struct MockDeps {
        dice: i32,
    }

    impl TurnServiceDeps for MockDeps {
        fn roll_dice(&self) -> i32 {
            self.dice
        }
        fn handle_event(&self, conn: &Connection, player_id: i32, tile_id: i32,) -> EventResult {
            EventResult::None
        }
    }


    fn setup() -> (Connection, SessionState) {
        let conn = Connection::open_in_memory().unwrap();
        let session = init_session(&conn).unwrap();
        (conn, session)
    }

   #[test]
    fn trans_no_owner_001_full_flow() {
        let (conn, mut session) = setup();

        conn.execute(
            "UPDATE players SET position=2, money=15 WHERE id=1",
            [],
        ).unwrap();

        conn.execute(
            "UPDATE tiles SET owner_id=NULL WHERE id=5",
            [],
        ).unwrap();

        session.current_turn_index = 1;

        set_will_buy(true);

        let repo = TurnRepoImpl;

        let result = process_turn_with_repo(
            &repo,
            &MockDeps { dice: 3 },
            &conn,
            &mut session,
        ).unwrap();

        assert_turn_result(&result, ExpectedTurnResult {
            player_id: 1,
            dice: 3,
            old_position: 2,
            new_position: 5,
            old_lap: 0,
            new_lap: 0,
            salary: 0,
            action_type: "purchase",
            action_amount: 9,
            owner_id: None,
            current_player_id: Some(2),
            game_finished: false,
            winner_id: None,
        });

        // DB 검증
        let owner: Option<i32> = conn.query_row(
            "SELECT owner_id FROM tiles WHERE id=5",
            [],
            |r| r.get(0),
        ).unwrap();

        assert_eq!(owner, Some(1));
    }



}
    


