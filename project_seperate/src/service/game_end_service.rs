#[derive(Clone, Debug)]
pub struct Player {
    pub id: i32,
    pub position: i32,
    pub money: i32,
    pub lap: i32,
    pub is_bankrupt: bool,
}

/// 게임 종료 결과
pub struct GameResult {
    pub is_finished: bool,
    pub winner_id: Option<i32>,
    pub rankings: Vec<(i32, i32)>, // (player_id, final_money)
}

/// 게임 종료 조건을 확인하고 결과를 계산
pub fn check_game_end(players: Vec<Player>) -> GameResult {
    let mut active_players = players
        .iter()
        .filter(|p| !p.is_bankrupt)
        .cloned()
        .collect::<Vec<_>>();

    if active_players.is_empty() {
        return GameResult {
            is_finished: true,
            winner_id: None,
            rankings: vec![],
        };
    }

    if active_players.len() == 1 {
        return GameResult {
            is_finished: true,
            winner_id: Some(active_players[0].id),
            rankings: vec![(active_players[0].id, active_players[0].money)],
        };
    }

    // 모든 생존 플레이어가 3바퀴 이상 돌았을 때 종료
    let finished = active_players.iter().all(|p| p.lap >= 3);

    if !finished {
        return GameResult {
            is_finished: false,
            winner_id: None,
            rankings: vec![],
        };
    }

    active_players.sort_by(|a, b| b.money.cmp(&a.money));

    // 우승자 결정
    let winner_id = active_players.first().map(|p| p.id);

    // 최종 랭킹 생성
    let rankings = active_players
        .iter()
        .map(|p| (p.id, p.money))
        .collect();

    GameResult {
        is_finished: true,
        winner_id,
        rankings,
    }
}