use crate::service::salary_service;

#[test]
fn test_no_salary_at_start() {
    let salary = calculate_salary(0, 0, 200000);
    assert_eq!(salary, 0);
}

#[test]
fn test_first_lap_salary() {
    let salary = calculate_salary(0, 1, 200000);
    assert_eq!(salary, 200000);
}

#[test]
fn test_multiple_laps_salary() {
    let salary = calculate_salary(1, 3, 200000);
    assert_eq!(salary, 400000);
}