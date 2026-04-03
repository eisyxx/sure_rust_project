#[cfg(test)]
mod fuzz_calculate_salary {
    use proptest::prelude::*;
    use crate::service::salary_service::calculate_salary;

    proptest! {
        #[test]
        fn fuzz_calculate_salary(
            prev_lap in -10i32..10,
            new_lap in -10i32..20,
            salary in 0i32..10000,
        ) {
            let result = calculate_salary(prev_lap, new_lap, salary);

            // 1. 결과는 0 또는 salary
            prop_assert!(result == 0 || result == salary);

            // 2. lap 증가하면 salary 지급
            if new_lap > prev_lap {
                prop_assert_eq!(result, salary);
            }

            // 3. lap 증가 안하면 salary 없음
            if new_lap <= prev_lap {
                prop_assert_eq!(result, 0);
            }

            // 4. 음수 salary 방지 (입력 조건상)
            prop_assert!(result >= 0);
        }
    }
}