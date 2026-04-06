/*
owner 있음 + 돈 충분 → PayToll
owner 있음 + 소유자 본인 → Skip
owner 있음 + 돈 부족 → Bankrupt
owner 없음 + 구매 안함 → Skip
owner 없음 + 구매 함 → Purchase
owner 없음 + 돈 부족 → NotEnoughMoney
tile_type == "start" → Skip
*/

#[cfg(test)]
mod tests {
    use crate::service::buy_property_service::{decide_buy_property, is_purchasable_tile, BuyResult};

    // owner 있음 + 돈 충분 → PayToll
    #[test]
    fn test_pay_toll() {
        let result = decide_buy_property(1, 100, 50, 20, Some(2), false, "land".to_string());

        match result {
            BuyResult::PayToll { owner_id, amount } => {
                assert_eq!(owner_id, 2);
                assert_eq!(amount, 20);
            }
            _ => panic!("Expected PayToll"),
        }
    }

    // owner 있음 + 소유자 본인 → Skip
    #[test]
    fn test_owner_is_self_should_skip() {
        let player_id = 1;

        let result = decide_buy_property(player_id, 1000, 200, 50,Some(player_id),true, "property".to_string());

        assert!(matches!(result, BuyResult::Skip));
    }
    
    // owner 있음 + 돈 부족 → Bankrupt
    #[test]
    fn test_bankrupt() {
        let result = decide_buy_property(1, 10, 50, 20, Some(2), false, "land".to_string());

        match result {
            BuyResult::Bankrupt { owner_id, paid } => {
                assert_eq!(owner_id, 2);
                assert_eq!(paid, 10);
            }
            _ => panic!("Expected Bankrupt"),
        }
    }

    // owner 없음 + 구매 안함 → Skip
    #[test]
    fn test_skip() {
        let result = decide_buy_property(1, 100, 50, 10, None, false, "land".to_string());

        assert!(matches!(result, BuyResult::Skip));
    }
    
    // owner 없음 + 구매 함 → Purchase
    #[test]
    fn test_purchase() {
        let result = decide_buy_property(1, 100, 50, 10, None, true, "land".to_string());

        match result {
            BuyResult::Purchase { price } => assert_eq!(price, 50),
            _ => panic!("Expected Purchase"),
        }
    }

    // owner 없음 + 돈 부족 → NotEnoughMoney
    #[test]
    fn test_not_enough_money() {
        let result = decide_buy_property(1, 10, 50, 10, None, true, "land".to_string());

        assert!(matches!(result, BuyResult::NotEnoughMoney));
    }
    
    // tile_type == "start" → Skip
    #[test]
    fn test_start_tile_skip() {
        let result = decide_buy_property(1, 100, 50, 10, None, true, "start".to_string());

        assert!(matches!(result, BuyResult::Skip));
    }

    // owner 없음 + 구매 가능 조건 충족 (price > 0, land 타입)
    #[test]
    fn test_purchasable_tile_true() {
        assert!(is_purchasable_tile(None, "land", 50));
    }
    
    // owner 있음 → 구매 불가
    #[test]
    fn test_purchasable_tile_has_owner() {
        assert!(!is_purchasable_tile(Some(1), "land", 50));
    }

    // tile_type == "event" → 구매 불가
    #[test]
    fn test_purchasable_tile_event() {
        assert!(!is_purchasable_tile(None, "event", 50));
    }

    // tile_type == "start" → 구매 불가
    #[test]
    fn test_purchasable_tile_start() {
        assert!(!is_purchasable_tile(None, "start", 50));
    }

    // price == 0 → 구매 불가
    #[test]
    fn test_purchasable_tile_zero_price() {
        assert!(!is_purchasable_tile(None, "land", 0));
    }
}