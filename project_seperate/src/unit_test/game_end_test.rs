/*
모든 플레이어 파산
한 명만 생존
3바퀴 미만 → 게임 계속
3바퀴 이상 → 게임 종료 + 랭킹/보상 계산
3바퀴 이상 → 게임 종료 + 파산자 포함 랭킹
*/

#[cfg(test)]
mod tests {
    use crate::service::game_end_service::{check_game_end, apply_rewards, evaluate_and_apply_game_end, Player};
    use rusqlite::{Connection};

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


    // apply_reward 테스트

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE players (
                id INTEGER PRIMARY KEY,
                position INTEGER DEFAULT 0,
                money INTEGER NOT NULL,
                lap INTEGER DEFAULT 1,
                is_bankrupt INTEGER DEFAULT 0,
                turn_order INTEGER DEFAULT 0
            )",
            [],
        ).unwrap();

        conn.execute(
            "CREATE TABLE transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                player_id INTEGER NOT NULL,
                type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                target TEXT NOT NULL,
                balance_before INTEGER NOT NULL,
                balance_after INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        conn
    }

    fn insert_player_simple(conn: &Connection, id: i32, money: i32) {
        conn.execute(
            "INSERT INTO players (id, position, money, lap, is_bankrupt, turn_order)
            VALUES (?1, 0, ?2, 1, 0, ?1)",
            rusqlite::params![id, money],
        ).unwrap();
    }
    fn insert_player_full(conn: &Connection, id: i32, money: i32, lap: i32, bankrupt: bool) {
        conn.execute(
            "INSERT INTO players (id, position, money, lap, is_bankrupt, turn_order)
            VALUES (?1, 0, ?2, ?3, ?4, ?1)",
            rusqlite::params![id, money, lap, bankrupt as i32],
        ).unwrap();
    }

    fn get_money(conn: &Connection, id: i32) -> i32 {
        conn.query_row(
            "SELECT money FROM players WHERE id = ?1",
            [id],
            |row| row.get(0),
        ).unwrap()
    }

    // rewards 비어있음
    #[test]
    fn test_apply_rewards_empty() {
        let conn = setup_db();

        insert_player_simple(&conn, 1, 100);

        apply_rewards(&conn, &[]).unwrap();

        assert_eq!(get_money(&conn, 1), 100); // 변화 없음
    }

    // 정상 동작
    #[test]
    fn test_apply_rewards_success() {
        let conn = setup_db();

        insert_player_simple(&conn, 1, 100);
        insert_player_simple(&conn, 2, 200);

        let rewards = vec![(1, 50), (2, 30)];

        apply_rewards(&conn, &rewards).unwrap();

        assert_eq!(get_money(&conn, 1), 150);
        assert_eq!(get_money(&conn, 2), 230);
    }

    // 중간 실패 (존재하지 않는 player)
    #[test]
    fn test_apply_rewards_error_midway() {
        let conn = setup_db();

        insert_player_simple(&conn, 1, 100);

        let rewards = vec![
            (1, 50),  // 성공
            (999, 30) // 존재하지 않음 → give_reward에서 에러 발생 가정
        ];

        let result = apply_rewards(&conn, &rewards);

        assert!(result.is_err());

        // 첫 번째는 적용됐는지 확인 (트랜잭션 없으면 반영됨)
        assert_eq!(get_money(&conn, 1), 150);
    }

    // evaluate_and_apply_game_end
    // 통합 검증에 가까움

    // 게임 종료 안됨
    #[test]
    fn test_not_finished_branch() {
        let conn = setup_db();

        // 종료 조건 안 맞게 (여러 명 생존)
        insert_player_full(&conn, 1, 100, 1, false);
        insert_player_full(&conn, 2, 100, 1, false);

        let result = evaluate_and_apply_game_end(&conn).unwrap();

        assert!(!result.game_finished);
        assert!(result.rankings.is_none());
    }

    // 게임 종료됨
    #[test]
    fn test_finished_branch() {
        let conn = setup_db();

        // 종료 조건 만들기 (한 명만 생존 등)
        insert_player_full(&conn, 1, 100, 3, false);
        insert_player_full(&conn, 2, 0, 1, true);

        let result = evaluate_and_apply_game_end(&conn).unwrap();

        assert!(result.game_finished);
        assert!(result.rankings.is_some());
    }

    // get_all_player 에러
    #[test]
    fn test_get_all_players_error_branch() {
        let conn = setup_db();

        conn.execute("DROP TABLE players", []).unwrap();

        let result = evaluate_and_apply_game_end(&conn);

        assert!(result.is_err());
    }

    // apply_rewards 에러
    #[test]
    fn test_apply_rewards_error_branch() {
        let conn = setup_db();

        insert_player_full(&conn, 1, 100, 3, false);

        conn.execute("DROP TABLE transactions", []).unwrap();

        let result = evaluate_and_apply_game_end(&conn);

        assert!(result.is_err());
    }

}