#[derive(Clone, Debug)]
pub struct MoveResult {
    pub new_position: i32,
    pub new_lap: i32,
    pub passed_start: bool,
}

// 플레이어의 이동 결과를 계산해 변경된 위치, 바퀴 수, 시작 지점 통과 여부를 반환
pub fn move_player(
    position: i32,
    lap: i32,
    dice: i32,
    total_tiles: i32,
) -> MoveResult {

    let mut new_position = position + dice;
    let mut new_lap = lap;
    let mut passed_start = false;

    if new_position >= total_tiles {
        new_position %= total_tiles;
        new_lap += 1;
        passed_start = true;
    }

    MoveResult {
        new_position,
        new_lap,
        passed_start,
    }
}