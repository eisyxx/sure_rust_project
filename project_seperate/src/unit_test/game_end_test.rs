/*
모든 플레이어 파산
한 명만 생존
3바퀴 미만 → 게임 계속
3바퀴 이상 → 게임 종료 + 랭킹/보상 계산
3바퀴 이상 → 게임 종료 + 파산자 포함 랭킹
*/

#[cfg(test)]
mod tests {
    use crate::service::game_end_service::{check_game_end, Player};

    // 플레이어 0명 상태
    #[test]
    fn test_empty_players() { 
        let result = check_game_end(vec![]);

        assert!(result.is_finished);
        assert!(result.winner_id.is_none());
        assert!(result.rankings.is_empty());
        assert!(result.rewards.is_empty());
    }

    // 플레이어는 존재하지만 전원 파산
    #[test]
    fn test_all_bankrupt() { 
        let players = vec![
            Player { id: 1, position: 0, money: 0, lap: 0, is_bankrupt: true },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert!(result.winner_id.is_none());
        assert!(result.rankings.is_empty());
        assert!(result.rewards.is_empty());
    }

    // 한 명만 생존
    #[test]
    fn test_single_winner() {
        let players = vec![
            Player { id: 1, position: 0, money: 100, lap: 0, is_bankrupt: false },
            Player { id: 2, position: 0, money: 0, lap: 0, is_bankrupt: true },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert_eq!(result.winner_id, Some(1));
        assert_eq!(result.rewards, vec![(1, 150)]);
    }

    // 3바퀴 이상 → 게임 종료 + 파산자 포함 랭킹
    #[test]
    fn test_finished_with_some_bankrupt_players() {
        let players = vec![
            Player { id: 1, position: 0, money: 300, lap: 3, is_bankrupt: false },
            Player { id: 2, position: 0, money: 100, lap: 3, is_bankrupt: false },
            Player { id: 3, position: 0, money: 0, lap: 1, is_bankrupt: true },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert_eq!(result.winner_id, Some(1));
        // 랭킹: 보상 반영 후 돈 기준 내림차순, 파산자 맨 뒤
        assert_eq!(result.rankings[0], (1, 300 + 150)); // 1등 보상
        assert_eq!(result.rankings[1], (2, 100 + 120)); // 2등 보상
        assert_eq!(result.rankings[2], (3, -1));         // 파산자
        // 보상: lap 기준 내림차순 상위 2명 (생존자만)
        assert_eq!(result.rewards, vec![(1, 150), (2, 120)]);
    }

    // 3바퀴 미만 → 게임 계속
    #[test]
    fn test_not_finished() {
        let players = vec![
            Player { id: 1, position: 0, money: 100, lap: 2, is_bankrupt: false },
            Player { id: 2, position: 0, money: 200, lap: 2, is_bankrupt: false },
        ];

        let result = check_game_end(players);

        assert!(!result.is_finished);
        assert!(result.winner_id.is_none());
        assert!(result.rankings.is_empty());
        assert!(result.rewards.is_empty());
    }

    // 3바퀴 이상, 파산자 없음 → 게임 종료 + 랭킹/보상 계산
    #[test]
    fn test_finished_all_lapped() {
        let players = vec![
            Player { id: 1, position: 0, money: 100, lap: 3, is_bankrupt: false },
            Player { id: 2, position: 0, money: 200, lap: 3, is_bankrupt: false },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        // 보상 반영 후: p1=100+150=250, p2=200+120=320 → p2가 1등
        assert_eq!(result.winner_id, Some(2));
        assert_eq!(result.rankings[0], (2, 320));
        assert_eq!(result.rankings[1], (1, 250));
    }
}

#[cfg(test)]
mod db_tests {
    use rusqlite::{params, Connection};
    use crate::service::game_end_service::{apply_rewards, evaluate_and_apply_game_end};

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE event_tiles (
                tile_id INTEGER PRIMARY KEY,
                event_type TEXT NOT NULL,
                amount INTEGER NOT NULL
            );

            -- player_repo.rs에서 사용하는 컬럼 반영
            CREATE TABLE players (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                position INTEGER NOT NULL,
                money INTEGER NOT NULL,
                lap INTEGER NOT NULL,
                turn_order INTEGER NOT NULL,
                is_bankrupt INTEGER NOT NULL
            );

            CREATE TABLE properties (
                tile_id INTEGER PRIMARY KEY,
                owner_id INTEGER,
                price INTEGER NOT NULL
            );

            CREATE TABLE fund (
                amount INTEGER NOT NULL
            );

            -- transcaction_repo.rs의 INSERT/SELECT 컬럼에 맞춤
            CREATE TABLE transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                player_id INTEGER NOT NULL,
                type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                target TEXT NOT NULL,
                balance_before INTEGER,
                balance_after INTEGER,
                created_at TEXT NOT NULL
            );
            "
        ).unwrap();
        conn
    }

    #[test]
    fn test_apply_rewards_updates_money_and_records_transactions() {
        let conn = setup_test_db();

        conn.execute(
            "INSERT INTO players (id, name, position, money, lap, turn_order, is_bankrupt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![1, "p1", 0, 100, 0, 1, 0],
        ).unwrap();

        conn.execute(
            "INSERT INTO players (id, name, position, money, lap, turn_order, is_bankrupt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![2, "p2", 0, 200, 0, 2, 0],
        ).unwrap();

        apply_rewards(&conn, &[(1, 150), (2, 120)]).unwrap();

        let p1: i32 = conn.query_row("SELECT money FROM players WHERE id = 1", [], |r| r.get(0)).unwrap();
        let p2: i32 = conn.query_row("SELECT money FROM players WHERE id = 2", [], |r| r.get(0)).unwrap();
        assert_eq!(p1, 250);
        assert_eq!(p2, 320);

        let tx_count: i32 = conn.query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0)).unwrap();
        assert_eq!(tx_count, 2);
    }

    #[test]
    fn test_evaluate_and_apply_game_end_finished() {
        let conn = setup_test_db();

        conn.execute(
            "INSERT INTO players (id, name, position, money, lap, turn_order, is_bankrupt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![1, "p1", 0, 100, 3, 1, 0],
        ).unwrap();

        conn.execute(
            "INSERT INTO players (id, name, position, money, lap, turn_order, is_bankrupt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![2, "p2", 0, 200, 3, 2, 0],
        ).unwrap();

        let result = evaluate_and_apply_game_end(&conn).unwrap();

        assert!(result.game_finished);
        assert_eq!(result.winner_id, Some(2));

        let rankings = result.rankings.expect("rankings should exist when finished");
        assert_eq!(rankings[0], (2, 320));
        assert_eq!(rankings[1], (1, 250));

        let p1: i32 = conn.query_row("SELECT money FROM players WHERE id = 1", [], |r| r.get(0)).unwrap();
        let p2: i32 = conn.query_row("SELECT money FROM players WHERE id = 2", [], |r| r.get(0)).unwrap();
        assert_eq!(p1, 250);
        assert_eq!(p2, 320);

        let tx_count: i32 = conn.query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0)).unwrap();
        assert_eq!(tx_count, 2);
    }

    #[test]
    fn test_evaluate_and_apply_game_end_not_finished() {
        let conn = setup_test_db();

        conn.execute(
            "INSERT INTO players (id, name, position, money, lap, turn_order, is_bankrupt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![1, "p1", 0, 100, 2, 1, 0],
        ).unwrap();

        conn.execute(
            "INSERT INTO players (id, name, position, money, lap, turn_order, is_bankrupt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![2, "p2", 0, 200, 2, 2, 0],
        ).unwrap();

        let result = evaluate_and_apply_game_end(&conn).unwrap();

        assert!(!result.game_finished);
        assert_eq!(result.winner_id, None);
        assert!(result.rankings.is_none());

        let p1: i32 = conn.query_row("SELECT money FROM players WHERE id = 1", [], |r| r.get(0)).unwrap();
        let p2: i32 = conn.query_row("SELECT money FROM players WHERE id = 2", [], |r| r.get(0)).unwrap();
        assert_eq!(p1, 100);
        assert_eq!(p2, 200);

        let tx_count: i32 = conn.query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0)).unwrap();
        assert_eq!(tx_count, 0);
    }
}
