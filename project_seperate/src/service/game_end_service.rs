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

    // 랭킹 생성: 파산자 마지막으로 보내기
    let mut rankings: Vec<(i32, i32)> = players
        .iter()
        .map(|p| (p.id, if p.is_bankrupt { -1 } else { p.money })) // 파산자는 -1로 처리
        .collect();

    // 돈 기준 내림차순 정렬 (파산자는 -1 → 맨 뒤)
    rankings.sort_by(|a, b| b.1.cmp(&a.1));

    // 우승자: 가장 높은 돈 가진 생존 플레이어
    let winner_id = rankings
        .iter()
        .find(|(id, money)| *money != -1)
        .map(|(id, _)| *id);

    GameResult {
        is_finished: finished,
        winner_id,
        rankings,
    }
}