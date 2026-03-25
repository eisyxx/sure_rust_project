/*
 * event_service.rs 단위 테스트
 * 
 * 테스트 대상: handle_event 함수
 * 테스트 전략: 각 이벤트 타입별 분기 커버리지 측정
 * 
 * 이벤트 타입:
 * 1. fund_add (사회복지기금): 돈 충분 / 부족
 * 2. tax_if_property (종합부동산세): 과세(돈충분/부족) / 스킵
 * 3. fund_take (기금 수령): 있음 / 없음
 * 4. Unknown: 알 수 없는 타입
 */

use mockall::predicate::*;
use mockall::mock;
use project::service::event_service::{handle_event, EventRepository, EventResult};

/// Mock EventRepository 생성
mock! {
    TestRepository {}
    
    impl EventRepository for TestRepository {
        fn get_event_info(&self, tile_id: i32) -> rusqlite::Result<(String, i32)>;
        fn get_player_money(&self, player_id: i32) -> rusqlite::Result<i32>;
        fn get_player_total_property_price(&self, player_id: i32) -> rusqlite::Result<i32>;
        fn get_fund_amount(&self) -> rusqlite::Result<i32>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // fund_add (사회복지기금) 테스트
    // ============================================

    /// fund_add: 돈이 충분한 경우
    #[test]
    fn test_fund_add_with_sufficient_money() {
        let mut repo = MockTestRepository::new();

        // 스텁: event_info 반환 (type="fund_add", amount=50)
        repo.expect_get_event_info()
            .with(eq(1))
            .returning(|_| Ok(("fund_add".to_string(), 50)));

        // 스텁: player 돈 반환 (100원)
        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(100));

        let result = handle_event(&repo, 1, 1);
        assert_eq!(result, EventResult::WelfareFund { amount: 50 });
    }

    /// fund_add: 돈이 부족한 경우
    #[test]
    fn test_fund_add_with_insufficient_money() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(1))
            .returning(|_| Ok(("fund_add".to_string(), 100)));

        // 돈이 부족 (30원 < 100원 필요)
        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(30));

        let result = handle_event(&repo, 1, 1);
        assert_eq!(result, EventResult::WelfareFundBankrupt { paid: 30 });
    }

    /// fund_add: 정확히 필요한 금액만 있는 경우
    #[test]
    fn test_fund_add_with_exact_money() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("fund_add".to_string(), 75)));

        repo.expect_get_player_money()
            .with(eq(2))
            .returning(|_| Ok(75));

        let result = handle_event(&repo, 2, 2);
        assert_eq!(result, EventResult::WelfareFund { amount: 75 });
    }

    /// fund_add: get_event_info 실패
    #[test]
    fn test_fund_add_get_event_info_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(1))
            .returning(|_| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 1);
        assert_eq!(result, EventResult::None);
    }

    /// fund_add: get_player_money 실패
    #[test]
    fn test_fund_add_get_player_money_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(1))
            .returning(|_| Ok(("fund_add".to_string(), 50)));

        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 1);
        assert_eq!(result, EventResult::None);
    }

    // ============================================
    // tax_if_property (종합부동산세) 테스트
    // ============================================

    /// tax_if_property: 부동산이 100 미만 (스킵)
    #[test]
    fn test_tax_if_property_below_threshold() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("tax_if_property".to_string(), 30)));

        // 부동산 가치 99 (< 100)
        repo.expect_get_player_total_property_price()
            .with(eq(1))
            .returning(|_| Ok(99));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::EstateTaxSkipped);
    }

    /// tax_if_property: 부동산이 100 이상이고 돈이 충분
    #[test]
    fn test_tax_if_property_sufficient_money() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("tax_if_property".to_string(), 30)));

        // 부동산 가치 150 (>= 100)
        repo.expect_get_player_total_property_price()
            .with(eq(1))
            .returning(|_| Ok(150));

        // 돈 충분 (200 >= 30)
        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(200));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::EstateTax { amount: 30 });
    }

    /// tax_if_property: 부동산이 100 이상이고 돈이 부족
    #[test]
    fn test_tax_if_property_insufficient_money() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("tax_if_property".to_string(), 50)));

        repo.expect_get_player_total_property_price()
            .with(eq(1))
            .returning(|_| Ok(500));

        // 돈 부족 (20 < 50)
        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(20));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::EstateTaxBankrupt { paid: 20 });
    }

    /// tax_if_property: 정확히 100인 경우
    #[test]
    fn test_tax_if_property_exactly_100() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("tax_if_property".to_string(), 40)));

        repo.expect_get_player_total_property_price()
            .with(eq(1))
            .returning(|_| Ok(100));

        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(100));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::EstateTax { amount: 40 });
    }

    /// tax_if_property: get_event_info 실패
    #[test]
    fn test_tax_if_property_get_event_info_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::None);
    }

    /// tax_if_property: get_player_total_property_price 실패 (unwrap_or(0) 동작)
    #[test]
    fn test_tax_if_property_get_property_price_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("tax_if_property".to_string(), 30)));

        // property_price 실패 -> unwrap_or(0) -> total = 0 -> 스킵
        repo.expect_get_player_total_property_price()
            .with(eq(1))
            .returning(|_| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::EstateTaxSkipped);
    }

    /// tax_if_property: get_player_money 실패
    #[test]
    fn test_tax_if_property_get_player_money_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(2))
            .returning(|_| Ok(("tax_if_property".to_string(), 30)));

        repo.expect_get_player_total_property_price()
            .with(eq(1))
            .returning(|_| Ok(500));

        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 2);
        assert_eq!(result, EventResult::None);
    }

    // ============================================
    // fund_take (기금 수령) 테스트
    // ============================================

    /// fund_take: 기금이 있는 경우
    #[test]
    fn test_fund_take_with_available_fund() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(3))
            .returning(|_| Ok(("fund_take".to_string(), 0)));

        // 기금 충분 (150)
        repo.expect_get_fund_amount()
            .returning(|| Ok(150));

        let result = handle_event(&repo, 1, 3);
        assert_eq!(result, EventResult::FundReceive { amount: 150 });
    }

    /// fund_take: 기금이 없는 경우 (0)
    #[test]
    fn test_fund_take_with_empty_fund() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(3))
            .returning(|_| Ok(("fund_take".to_string(), 0)));

        // 기금 없음 (0)
        repo.expect_get_fund_amount()
            .returning(|| Ok(0));

        let result = handle_event(&repo, 1, 3);
        assert_eq!(result, EventResult::FundReceiveEmpty);
    }

    /// fund_take: 기금이 음수인 경우 (에러 케이스)
    #[test]
    fn test_fund_take_with_negative_fund() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(3))
            .returning(|_| Ok(("fund_take".to_string(), 0)));

        // 기금 음수 (< 0)
        repo.expect_get_fund_amount()
            .returning(|| Ok(-100));

        let result = handle_event(&repo, 1, 3);
        assert_eq!(result, EventResult::FundReceiveEmpty);
    }

    /// fund_take: get_event_info 실패
    #[test]
    fn test_fund_take_get_event_info_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(3))
            .returning(|_| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 3);
        assert_eq!(result, EventResult::None);
    }

    /// fund_take: get_fund_amount 실패
    #[test]
    fn test_fund_take_get_fund_amount_fails() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(3))
            .returning(|_| Ok(("fund_take".to_string(), 0)));

        repo.expect_get_fund_amount()
            .returning(|| Err(rusqlite::Error::InvalidQuery));

        let result = handle_event(&repo, 1, 3);
        assert_eq!(result, EventResult::None);
    }

    // ============================================
    // Unknown 이벤트 타입 테스트
    // ============================================

    /// Unknown event type
    #[test]
    fn test_unknown_event_type() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(4))
            .returning(|_| Ok(("unknown_event".to_string(), 100)));

        let result = handle_event(&repo, 1, 4);
        assert_eq!(result, EventResult::None);
    }

    /// Empty string event type
    #[test]
    fn test_empty_event_type() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(5))
            .returning(|_| Ok(("".to_string(), 100)));

        let result = handle_event(&repo, 1, 5);
        assert_eq!(result, EventResult::None);
    }

    // ============================================
    // 엣지 케이스 테스트
    // ============================================

    /// 여러 플레이어 테스트 (플레이어 ID 2)
    #[test]
    fn test_different_player_id() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(10))
            .returning(|_| Ok(("fund_add".to_string(), 100)));

        repo.expect_get_player_money()
            .with(eq(5))
            .returning(|_| Ok(200));

        let result = handle_event(&repo, 5, 10);
        assert_eq!(result, EventResult::WelfareFund { amount: 100 });
    }

    /// 0원 이벤트 처리
    #[test]
    fn test_zero_amount_event() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(1))
            .returning(|_| Ok(("fund_add".to_string(), 0)));

        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(100));

        let result = handle_event(&repo, 1, 1);
        assert_eq!(result, EventResult::WelfareFund { amount: 0 });
    }

    /// 큰 금액 테스트
    #[test]
    fn test_large_amount() {
        let mut repo = MockTestRepository::new();

        repo.expect_get_event_info()
            .with(eq(1))
            .returning(|_| Ok(("fund_add".to_string(), 1000000)));

        repo.expect_get_player_money()
            .with(eq(1))
            .returning(|_| Ok(2000000));

        let result = handle_event(&repo, 1, 1);
        assert_eq!(result, EventResult::WelfareFund { amount: 1000000 });
    }
}
