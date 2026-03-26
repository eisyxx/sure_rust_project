/*
모든 플레이어 파산
한 명만 생존
3바퀴 미만 → 게임 계속
3바퀴 이상 → 게임 종료 + 랭킹 정렬
3바퀴 이상 → 게임 종료 + 랭킹 정렬 (파산자 존재)
*/

#[cfg(test)]
mod tests {
    use crate::service::game_end_service::{check_game_end, Player};

    #[test]
    fn test_all_bankrupt() {
        let players = vec![
            Player { id: 1, position: 0, money: 0, lap: 0, is_bankrupt: true },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert!(result.winner_id.is_none());
    }

    #[test]
    fn test_single_winner() {
        let players = vec![
            Player { id: 1, position: 0, money: 100, lap: 0, is_bankrupt: false },
            Player { id: 2, position: 0, money: 0, lap: 0, is_bankrupt: true },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert_eq!(result.winner_id, Some(1));
    }

    #[test]
    fn test_finished_with_some_bankrupt_players() {
        let players = vec![
            Player { id: 1, position: 0, money: 300, lap: 3, is_bankrupt: false },
            Player { id: 2, position: 0, money: 100, lap: 3, is_bankrupt: false },
            Player { id: 3, position: 0, money: 0, lap: 1, is_bankrupt: true }, // 파산자
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert_eq!(result.winner_id, Some(1)); // 우승자 (돈 가장 많은 생존자)
        assert_eq!(result.rankings.len(), 3); // 랭킹 검증
        assert_eq!(result.rankings[0], (1, 300)); // 1등: player 1 (300)
        assert_eq!(result.rankings[1], (2, 100)); // 2등: player 2 (100)
        assert_eq!(result.rankings[2], (3, -1)); // 3등: 파산자 → -1 처리되어 맨 뒤
    }

    #[test]
    fn test_not_finished() {
        let players = vec![
            Player { id: 1, position: 0, money: 100, lap: 2, is_bankrupt: false },
            Player { id: 2, position: 0, money: 200, lap: 2, is_bankrupt: false },
        ];

        let result = check_game_end(players);

        assert!(!result.is_finished);
    }

    #[test]
    fn test_ranking() {
        let players = vec![
            Player { id: 1, position: 0, money: 100, lap: 3, is_bankrupt: false },
            Player { id: 2, position: 0, money: 200, lap: 3, is_bankrupt: false },
        ];

        let result = check_game_end(players);

        assert!(result.is_finished);
        assert_eq!(result.rankings[0].0, 2);
    }
}