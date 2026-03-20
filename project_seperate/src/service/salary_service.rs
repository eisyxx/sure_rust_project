pub fn calculate_salary(
    prev_lap: i32,
    new_lap: i32,
    salary: i32,
) -> i32 {
    if new_lap > prev_lap {
        salary
    } else {
        0
    }
}