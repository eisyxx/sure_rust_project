use rusqlite::Connection;
use crate::repository::player_repo::{get_all_players, give_reward};

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
    pub rankings: Vec<(i32, i32)>,
    pub rewards: Vec<(i32, i32)>,
}

/// 게임 종료 조건을 확인하고, 종료 시 랭킹/우승자/보상까지 계산
pub fn check_game_end(players: Vec<Player>) -> GameResult {
    let active_players: Vec<_> = players
        .iter()
        .filter(|p| !p.is_bankrupt)
        .cloned()
        .collect();

    // 종료 여부 판단
    let is_finished = if active_players.is_empty() {
        true
    } else if active_players.len() == 1 {
        true
    } else {
        active_players.iter().all(|p| p.lap >= 3)
    };

    if !is_finished {
        return GameResult {
            is_finished: false,
            winner_id: None,
            rankings: vec![],
            rewards: vec![],
        };
    }

    // 전원 파산 예외 처리
    if active_players.is_empty() {
        return GameResult {
            is_finished: true,
            winner_id: None,
            rankings: vec![],
            rewards: vec![],
        };
    }

    // 보상 계산 (생존자 기준)
    let mut rewards: Vec<(i32, i32)> = vec![];

    if active_players.len() == 1 {
        // 생존자 1명 → 150 지급
        rewards.push((active_players[0].id, 150));
    } else {
        // 일반 케이스 → lap 기준 상위 3명
        let mut sorted_for_reward = active_players.clone();
        sorted_for_reward.sort_by(|a, b| b.lap.cmp(&a.lap));

        let reward_amounts = [150, 120, 80];

        rewards = sorted_for_reward
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < reward_amounts.len())
            .map(|(i, p)| (p.id, reward_amounts[i]))
            .collect();
    }

    // 랭킹 계산 (전체 플레이어 기준)
    let mut rankings: Vec<(i32, i32)> = players
        .iter()
        .map(|p| {
            if p.is_bankrupt {
                (p.id, -1)
            } else {
                let bonus = rewards
                    .iter()
                    .find(|(rid, _)| *rid == p.id)
                    .map(|(_, amt)| *amt)
                    .unwrap_or(0);

                (p.id, p.money + bonus)
            }
        })
        .collect();

    rankings.sort_by(|a, b| b.1.cmp(&a.1));

    // winner 계산 (rankings 기준)
    let winner_id = rankings
        .iter()
        .find(|(_, money)| *money != -1)
        .map(|(id, _)| *id);

    GameResult {
        is_finished: true,
        winner_id,
        rankings,
        rewards,
    }
}

/// 게임 종료 시 보상을 DB에 반영
pub fn apply_rewards(conn: &Connection, rewards: &[(i32, i32)]) -> rusqlite::Result<()> {
    for (player_id, amount) in rewards {
        give_reward(conn, *player_id, *amount)?;
    }
    Ok(())
}

/// 턴 종료 후 게임 종료 여부를 판단하고, 종료 시 보상까지 DB에 반영하는 서비스 함수.
/// DB에서 전체 플레이어를 조회 → 게임 종료 조건 확인 → 종료 시 보상 반영을 한 번의 호출로 처리한다.
pub struct AdvanceTurnResult {
    pub game_finished: bool,
    pub winner_id: Option<i32>,
    pub rankings: Option<Vec<(i32, i32)>>,
}

pub fn evaluate_and_apply_game_end(conn: &Connection) -> rusqlite::Result<AdvanceTurnResult> {
    let all_rows = get_all_players(conn)?;
    let game_players: Vec<Player> = all_rows
        .iter()
        .map(|row| Player {
            id: row.id,
            position: row.position,
            money: row.money,
            lap: row.lap,
            is_bankrupt: row.is_bankrupt,
        })
        .collect();

    let game_result = check_game_end(game_players);

    if game_result.is_finished {
        apply_rewards(conn, &game_result.rewards)?;
    }

    Ok(AdvanceTurnResult {
        game_finished: game_result.is_finished,
        winner_id: game_result.winner_id,
        rankings: if game_result.is_finished {
            Some(game_result.rankings)
        } else {
            None
        },
    })
}