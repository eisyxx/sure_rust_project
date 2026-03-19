use crate::service::movement_service;

#[test]
fn test_move_no_lap() {
    let result = move_player(3, 1, 2, 20);

    assert_eq!(result.new_position, 5);
    assert_eq!(result.new_lap, 1);
    assert!(!result.passed_start);
}

#[test]
fn test_move_with_lap() {
    let result = move_player(18, 1, 5, 20);

    assert_eq!(result.new_position, 3);
    assert_eq!(result.new_lap, 2);
    assert!(result.passed_start);
}