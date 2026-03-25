/*
owner 있음 + 돈 충분 → PayToll
owner 있음 + 돈 부족 → Bankrupt
owner 없음 + 구매 안함 → Skip
owner 없음 + 구매 함 → Purchase
owner 없음 + 돈 부족 → NotEnoughMoney
tile_type == "event" → Skip
tile_type == "start" → Skip
*/

#[cfg(test)]
mod tests {
    use project::service::buy_property_service::{decide_buy_property, BuyResult};

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

    #[test]
    fn test_skip_not_buy() {
        let result = decide_buy_property(1, 100, 50, 10, None, false, "land".to_string());

        assert!(matches!(result, BuyResult::Skip));
    }

    #[test]
    fn test_purchase_success() {
        let result = decide_buy_property(1, 100, 50, 10, None, true, "land".to_string());

        match result {
            BuyResult::Purchase { price } => assert_eq!(price, 50),
            _ => panic!("Expected Purchase"),
        }
    }

    #[test]
    fn test_not_enough_money() {
        let result = decide_buy_property(1, 10, 50, 10, None, true, "land".to_string());

        assert!(matches!(result, BuyResult::NotEnoughMoney));
    }

    #[test]
    fn test_event_tile_skip() {
        let result = decide_buy_property(1, 100, 50, 10, None, true, "event".to_string());

        assert!(matches!(result, BuyResult::Skip));
    }

    #[test]
    fn test_start_tile_skip() {
        let result = decide_buy_property(1, 100, 50, 10, None, true, "start".to_string());

        assert!(matches!(result, BuyResult::Skip));
    }
}