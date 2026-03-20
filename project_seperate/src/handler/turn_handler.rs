use rusqlite::Connection;

use crate::repository::{
    player_repo::get_all_players,
    property_repo::get_owner,
    tile_repo::get_tile_info,
};

use crate::service::turn_service::{process_turn, TurnInput};
use crate::service::turn_execute_service::apply_turn_result;
use crate::service::game_end_service::{check_game_end, Player as GamePlayer};

/// 한 턴을 진행하는 핸들러
/// 자동으로 현재 플레이어 순서대로 진행
pub fn handle_turn(conn: &Connection, current_turn_index: &mut usize) -> rusqlite::Result<bool> {
    // 1. 모든 플레이어 가져오기 (파산자 제외)
    let mut players = get_all_players(conn)?
        .into_iter()
        .filter(|p| !p.is_bankrupt)
        .collect::<Vec<_>>();
    println!("플레이어 가져오기 성공"); //테스트용

    if players.is_empty() {
        println!("게임에 남은 플레이어가 없습니다!");
        return Ok(true); // 게임 종료
    }

    // 2. 현재 턴 플레이어 선택 (순환)
    if *current_turn_index >= players.len() {
        *current_turn_index = 0;
    }
    let current_player = &players[*current_turn_index];

    println!("Player {} 턴 시작!", current_player.id);

    // 3. 타일 정보 가져오기
    let (tile_price, tile_toll, till_owner, tile_type) = get_tile_info(conn, current_player.position)?;
    let tile_owner = get_owner(conn, current_player.position)?;
    println!("타일정보 가져오기 성공"); //테스트용

    // 4. 구매 여부 결정 (임시: 항상 구매)
    let will_buy = true;

    // 5. TurnInput 생성
    let turn_input = TurnInput {
        player_id: current_player.id,
        position: current_player.position,
        lap: current_player.lap,
        money: current_player.money,
        total_tiles: 24, // 고정 맵
        tile_price,
        tile_toll,
        owner: tile_owner,
        will_buy,
        tile_type: tile_type.clone(),
    };
    println!("TurnInput 생성 성공"); //테스트용
    println!(
    "DEBUG: player_id={}, pos={}, tile_price={}, tile_type={}, owner={:?}",
    current_player.id,
    current_player.position,
    tile_price,
    tile_type,
    tile_owner
    ); //테스트용

    // 6. 턴 처리
    let turn_result = process_turn(turn_input,conn);
    println!("process_turn 성공"); //테스트용

    // 7. DB에 반영
    apply_turn_result(conn, current_player.id, &turn_result)?;
    println!("apply_turn_result 성공"); //테스트용

    println!(
        "Player {} 이동: {} -> {}, lap: {}, action: {:?}, salary: {}",
        current_player.id,
        current_player.position,
        turn_result.new_position,
        turn_result.new_lap,
        turn_result.action,
        turn_result.salary,
    );

    // 8. 게임 종료 체크
    let all_players = get_all_players(conn)?
        .into_iter()
        .map(|p| GamePlayer {
            id: p.id,
            position: p.position,
            money: p.money,
            lap: p.lap,
            is_bankrupt: p.is_bankrupt,
        })
        .collect::<Vec<_>>();

    let game_result = check_game_end(all_players);

    if game_result.is_finished {
        println!("게임 종료! 승자: {:?}", game_result.winner_id);
        println!("최종 랭킹: {:?}", game_result.rankings);
        return Ok(true); // 게임 종료
    }

    // 9. 다음 턴으로 순서 이동
    *current_turn_index += 1;

    Ok(false) // 게임 계속 진행
}