/*
일반 이동 (lap 증가 없음)
lap 증가 발생
*/

#[cfg(test)]
mod tests {
    use crate::service::movement_service::move_player;

    #[test]
    fn test_move_without_lap() {
        let result = move_player(3, 0, 2, 10);

        assert_eq!(result.new_position, 5);
        assert_eq!(result.new_lap, 0);
    }

    #[test]
    fn test_move_with_lap() {
        let result = move_player(9, 0, 3, 10);

        assert_eq!(result.new_position, 2);
        assert_eq!(result.new_lap, 1);
    }
}