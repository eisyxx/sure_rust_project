/*
주사위 값 범위 테스트 (1~6)
*/

#[cfg(test)]
mod tests {
    use crate::service::roll_dice_service::roll_dice;

    // 주사위 값 범위 테스트 (1~6)
    #[test]
    fn test_dice_range() {
        let result = roll_dice();
        assert!(result >= 1 && result <= 6);
    }
}