/*
** DB 필요**
fund_add → 돈 충분 / 부족
tax_if_property → 과세 / 스킵
fund_take → 있음 / 없음
*/

mod common;

#[cfg(test)]
mod tests {
    use project::service::event_service::{handle_event,EventResult};
    use crate::common::db::setup_db;
    use rusqlite::Connection;

    #[test]
    fn test_event() {
        let conn = setup_db();

        conn.execute("INSERT INTO players VALUES (1, 100)", []).unwrap();

        // 테스트 실행
    }

    #[test]
    fn test_welfare_fund() {
        let conn = setup_db();

        conn.execute("INSERT INTO events VALUES (1, 'fund_add', 50)", []).unwrap();
        conn.execute("INSERT INTO players VALUES (1, 100)", []).unwrap();

        let result = handle_event(&conn, 1, 1);

        assert!(matches!(result, EventResult::WelfareFund { .. }));
    }
}