use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
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
}

/// 게임 종료 조건을 확인하고 결과를 계산
pub fn check_game_end(players: Vec<Player>) -> GameResult {
    let active_players = players
        .iter()
        .filter(|p| !p.is_bankrupt)
        .cloned()
        .collect::<Vec<_>>();

    if active_players.is_empty() {
        return GameResult {
            is_finished: true,
        };
    }

    if active_players.len() == 1 {
        return GameResult {
            is_finished: true,
        };
    }

    // 모든 생존 플레이어가 3바퀴 이상 돌았을 때 종료
    let finished = active_players.iter().all(|p| p.lap >= 3);

    GameResult {
        is_finished: finished,
    }
}