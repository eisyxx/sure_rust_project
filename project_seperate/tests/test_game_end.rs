use crate::service::game_end_service;

#[test]
fn test_game_end() {
    let players = vec![
        Player { id: 1, position: 2, money: 500, lap: 3, is_bankrupt: false },
        Player { id: 2, position: 5, money: 600, lap: 2, is_bankrupt: false },
        Player { id: 3, position: 1, money: 400, lap: 3, is_bankrupt: false },
        Player { id: 4, position: 3, money: 300, lap: 1, is_bankrupt: false },
    ];

    let result = check_game_end(players);

    assert!(result.is_finished);
    assert_eq!(result.winner_id, Some(1)); // 계산 결과에 따라 변경 가능
}