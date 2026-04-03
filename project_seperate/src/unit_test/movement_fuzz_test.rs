#[cfg(test)]
mod fuzz_move_player {
    use proptest::prelude::*;
    use crate::service::movement_service::move_player;

    proptest! {
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