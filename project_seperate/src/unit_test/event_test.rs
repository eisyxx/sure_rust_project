/*
** DB 필요**
fund_add → 돈 충분 / 부족
tax_if_property → 과세 / 스킵
fund_take → 있음 / 없음
*/

#[cfg(test)]
mod tests {
    use crate::service::event_service::{handle_event, EventResult};
    use super::super::common::db::setup_db;

    #[test]
    fn test_event() {
        let conn = setup_db();

        conn.execute("INSERT INTO players VALUES (1, 100)", []).unwrap();

        // 테스트 실행
    }

}