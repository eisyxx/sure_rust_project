/*
주사위 값 범위 테스트 (1~6)
*/

#[cfg(test)]
mod tests {
    use project::service::roll_dice_service::roll_dice;

    #[test]
    fn test_dice_range() {
        let result = roll_dice();
        assert!(result >= 1 && result <= 6);
    }
}