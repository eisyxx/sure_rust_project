use rusqlite::Connection;

use crate::repository::{
    player_repo::{update_money, update_position_and_lap, bankrupt},
    property_repo::{set_owner, reset_owner_for_player},
    transcaction_repo::record_transaction,
};

use crate::service::turn_service::{TurnResult, TurnAction};

// process_turn 함수 실행 결과를 DB에 반영하는 함수
pub fn apply_turn_result(
    conn: &Connection,
    player_id: i32,
    result: &TurnResult,
) -> rusqlite::Result<()> {

    // 위치 + lap(바퀴 수) 업데이트
    update_position_and_lap(
        conn,
        player_id,
        result.new_position,
        result.new_lap,
    )?;

    // 월급 처리 (입금 후 내역 기록)
    if result.salary > 0 {
        update_money(conn, player_id, result.salary)?;

        record_transaction(
            conn,
            player_id,
            "deposit",
            result.salary,
            "salary",
        )?;
    }

    // 액션 처리
    match &result.action {

        // 토지 구매
        TurnAction::Purchase { price } => {
            update_money(conn, player_id, -*price)?;

            record_transaction(
                conn,
                player_id,
                "withdraw",
                *price,
                &format!("tile{}_purchase", result.new_position),
            )?;

            set_owner(
                conn,
                result.new_position,
                player_id,
                *price,
            )?;
        }

        // 통행료 지급
        TurnAction::PayToll { owner_id, amount } => {
            // 잔액 출금
            update_money(conn, player_id, -*amount)?;
            record_transaction(
                conn,
                player_id,
                "withdraw",
                *amount,
                &format!("toll_to_{}", owner_id),
            )?;

            // 토지 소유자 잔액 증가
            update_money(conn, *owner_id, *amount)?;
            record_transaction(
                conn,
                *owner_id,
                "deposit",
                *amount,
                &format!("toll_from_{}", player_id),
            )?;
        }

        // 파산
        TurnAction::Bankrupt { owner_id, paid } => {
            // 파산 플레이어 잔액을 먼저 차감해야 거래 전/후 잔액이 올바르게 기록된다.
            update_money(conn, player_id, -*paid)?;

            // 잔액을 전부 토지 소유자에게 지급
            update_money(conn, *owner_id, *paid)?;
            record_transaction(
                conn,
                *owner_id,
                "deposit",
                *paid,
                &format!("bankrupt_from_{}", player_id),
            )?;

            // 출금 내역 기록
            record_transaction(
                conn,
                player_id,
                "withdraw",
                *paid,
                &format!("bankrupt_to_{}", owner_id),
            )?;

            // 소유했던 토지 초기화
            reset_owner_for_player(conn, player_id)?;

            // 파산 처리
            bankrupt(conn, player_id)?;
        }

        // 이벤트 A: 사회복지기금
        TurnAction::EventWelfareFund { amount } => {
            use crate::repository::event_repo::add_fund;

            // 돈 차감
            update_money(conn, player_id, -*amount)?;

            // 기금 증가
            add_fund(conn, *amount)?;

            // 거래 기록
            record_transaction(
                conn,
                player_id,
                "withdraw",
                *amount,
                "welfare_fund",
            )?;
        }

        // 이벤트 A: 파산
        TurnAction::EventWelfareFundBankrupt { paid } => {
            use crate::repository::event_repo::add_fund;

            // 가진 돈 전부 기금으로
            add_fund(conn, *paid)?;

            // 거래 기록
            record_transaction(
                conn,
                player_id,
                "withdraw",
                *paid,
                "welfare_fund_bankrupt",
            )?;

            // 토지 초기화
            reset_owner_for_player(conn, player_id)?;

            // 파산 처리
            bankrupt(conn, player_id)?;
        }

        // 이벤트 C: 기금 수령
        TurnAction::EventFundReceive { amount } => {
            use crate::repository::event_repo::reset_fund;

            // 플레이어 돈 증가
            update_money(conn, player_id, *amount)?;

            // 거래 기록
            record_transaction(
                conn,
                player_id,
                "deposit",
                *amount,
                "fund_receive",
            )?;

            // 기금 초기화
            reset_fund(conn)?;
        }

        TurnAction::None => {}

        TurnAction::EstateTaxSkipped => {}

        // 종부세 납부
        TurnAction::EstateTax { amount } => {
            update_money(conn, player_id, -*amount)?;

            record_transaction(
                conn,
                player_id,
                "withdraw",
                *amount,
                "estate_tax",
            )?;
        }
    }

    Ok(())
}