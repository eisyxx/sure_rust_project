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
pub fn check_game_end(mut players: Vec<Player>) -> GameResult {
    // 3바퀴 이상이고 파산하지 않은 플레이어 존재 확인
    let finished = players.iter().any(|p| p.lap >= 3 && !p.is_bankrupt);

    //없다면 게임 계속 진행
    if !finished {
        return GameResult {
            is_finished: false,
            winner_id: None,
            rankings: vec![],
        };
    }

    // 시작점 기준 거리 정렬
    players.sort_by(|a, b| {
    // 1. lap 먼저 비교
    match b.lap.cmp(&a.lap) {
        std::cmp::Ordering::Equal => {
            // 2. 같으면 position 비교
            b.position.cmp(&a.position)
        }
        other => other,
    }
    });

    // 상금 지급 (150 / 120 / 80)
    let rewards = [150, 120, 80];

    for (i, player) in players.iter_mut().enumerate() {
        if i < rewards.len() && !player.is_bankrupt {
            player.money += rewards[i];
        }
    }

    // 최종 금액 기준 정렬
    players.sort_by(|a, b| b.money.cmp(&a.money));

    // 우승자 결정
    let winner_id = players.first().map(|p| p.id);

    // 최종 랭킹 생성
    let rankings = players
        .iter()
        .map(|p| (p.id, p.money))
        .collect();

    GameResult {
        is_finished: true,
        winner_id,
        rankings,
    }
}