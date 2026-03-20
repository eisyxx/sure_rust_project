use crate::service::turn_service;

#[test]
fn test_full_turn() {
    let input = TurnInput {
        player_id: 1,
        position: 18,
        lap: 0,
        money: 500000,
        dice: 5,
        total_tiles: 20,
        tile_price: 100000,
        tile_toll: 20000,
        owner: None,
        will_buy: true,
    };

    let result = process_turn(input);

    assert_eq!(result.new_lap, 1);
    assert!(result.salary > 0);
}