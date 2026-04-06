#[cfg(test)]
mod fuzz_move_player {
    use proptest::prelude::*;
    use crate::service::movement_service::move_player;

    proptest! {
        // 게임 규칙 보장하는 입력 테스트
        #[test]
        fn fuzz_move_player_valid_input(
            total_tiles in 2i32..50,
            position in 0i32..50,
            lap in 0i32..10,
            dice in 1i32..7,
        ) {
            // 게임 규칙 보장
            prop_assume!(position < total_tiles);

            let result = move_player(position, lap, dice, total_tiles);

            // 1. 위치 범위
            prop_assert!(result.new_position >= 0);
            prop_assert!(result.new_position < total_tiles);

            // 2. 정확한 위치 계산
            let expected_position = (position + dice) % total_tiles;
            prop_assert_eq!(result.new_position, expected_position);

            // 3. lap 정확성
            if position + dice >= total_tiles {
                prop_assert_eq!(result.new_lap, lap + 1);
            } else {
                prop_assert_eq!(result.new_lap, lap);
            }
        }

        // 게임 규칙 외 비정상 입력 테스트
        #[test]
        fn fuzz_move_player(
            position in -100i32..100,
            lap in -10i32..10,
            dice in -50i32..50,
            total_tiles in 1i32..50, // 0 제외 (panic 방지)
        ) {
            let result = move_player(position, lap, dice, total_tiles);

            // 1. 위치 범위 보장 (핵심)
            prop_assert!(result.new_position < total_tiles);

            // 현재 코드에서는 음수 가능 → 일부러 체크
            // 이거 실패하면 버그
            prop_assert!(result.new_position >= 0);

            // 2. lap 감소 금지
            prop_assert!(result.new_lap >= lap);

            // 3. lap 증가 조건 검증
            if position + dice >= total_tiles {
                prop_assert_eq!(result.new_lap, lap + 1);
            } else {
                prop_assert_eq!(result.new_lap, lap);
            }
        }
    }
}