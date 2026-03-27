/*
lap 증가 → 월급 지급
lap 동일 → 월급 없음
*/

#[cfg(test)]
mod tests {
    use crate::service::salary_service::calculate_salary;

    #[test]
    fn test_salary_given() {
        let result = calculate_salary(0, 1, 20);
        assert_eq!(result, 20);
    }

    #[test]
    fn test_salary_not_given() {
        let result = calculate_salary(1, 1, 20);
        assert_eq!(result, 0);
    }
}