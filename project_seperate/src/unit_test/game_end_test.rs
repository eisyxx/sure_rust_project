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