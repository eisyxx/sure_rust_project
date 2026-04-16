use rusqlite::Connection;

use crate::repository::{
    player_repo::{update_money, update_position_and_lap},
    property_repo::{set_owner},
    transcaction_repo::record_transaction,
};

use crate::service::turn_service::{TurnResult, TurnAction};
use crate::service::traits::TurnExecuteRepo;


// process_turn 함수 실행 결과를 DB에 반영하는 함수 (DI 버전)
pub fn apply_turn_result_with_repo<R: TurnExecuteRepo>(
    repo: &R,
    conn: &Connection,
    player_id: i32,
    result: &TurnResult,
) -> rusqlite::Result<()> {

    // 위치 + lap(바퀴 수) 업데이트
    repo.update_position_and_lap(
        conn,
        player_id,
        result.new_position,
        result.new_lap,
    )?;

    // 월급 처리 (입금 후 내역 기록)
    if result.salary > 0 {
        repo.update_money(conn, player_id, result.salary)?;

        repo.record_transaction(
            conn,
            player_id,
            "deposit",
            result.salary,
            "salary",
        )?;
    }

    // 액션 처리
    match &result.action {

        // 통행료 지급
        TurnAction::PayToll { owner_id, amount } => {
            // 잔액 출금
            repo.update_money(conn, player_id, -*amount)?;
            repo.record_transaction(
                conn,
                player_id,
                "withdraw",
                *amount,
                &format!("toll_to_{}", owner_id),
            )?;

            // 토지 소유자 잔액 증가
            repo.update_money(conn, *owner_id, *amount)?;
            repo.record_transaction(
                conn,
                *owner_id,
                "deposit",
                *amount,
                &format!("toll_from_{}", player_id),
            )?;
        }

        // 파산
        TurnAction::Bankrupt { owner_id, paid } => {
            repo.update_money(conn, player_id, -*paid)?;

            repo.update_money(conn, *owner_id, *paid)?;
            repo.record_transaction(
                conn,
                *owner_id,
                "deposit",
                *paid,
                &format!("bankrupt_from_{}", player_id),
            )?;

            repo.record_transaction(
                conn,
                player_id,
                "withdraw",
                *paid,
                &format!("bankrupt_to_{}", owner_id),
            )?;

            repo.reset_owner_for_player(conn, player_id)?;

            repo.bankrupt(conn, player_id)?;
        }

        // 이벤트 A: 사회복지기금
        TurnAction::EventWelfareFund { amount } => {
            repo.update_money(conn, player_id, -*amount)?;

            repo.add_fund(conn, *amount)?;

            repo.record_transaction(
                conn,
                player_id,
                "withdraw",
                *amount,
                "welfare_fund",
            )?;
        }

        // 이벤트 A: 파산
        TurnAction::EventWelfareFundBankrupt { paid } => {
            repo.add_fund(conn, *paid)?;

            repo.record_transaction(
                conn,
                player_id,
                "withdraw",
                *paid,
                "welfare_fund_bankrupt",
            )?;

            repo.reset_owner_for_player(conn, player_id)?;

            repo.bankrupt(conn, player_id)?;
        }

        // 이벤트 C: 기금 없음
        TurnAction::FundReceiveEmpty => {}

        // 이벤트 C: 기금 수령
        TurnAction::EventFundReceive { amount } => {
            repo.update_money(conn, player_id, *amount)?;

            repo.record_transaction(
                conn,
                player_id,
                "deposit",
                *amount,
                "welfare_fund_receive",
            )?;

            repo.reset_fund(conn)?;
        }

        TurnAction::None => {}

        TurnAction::EstateTaxSkipped => {}

        // 이벤트 B: 종합부동산세
        TurnAction::EstateTax { amount } => {
            repo.update_money(conn, player_id, -*amount)?;

            repo.record_transaction(
                conn,
                player_id,
                "withdraw",
                *amount,
                "estate_tax",
            )?;
        }

        // 이벤트 B: 파산
        TurnAction::EstateTaxBankrupt { paid } => {
            repo.update_money(conn, player_id, -*paid)?;

            repo.record_transaction(
                conn,
                player_id,
                "withdraw",
                *paid,
                "estate_tax_bankrupt",
            )?;

            repo.reset_owner_for_player(conn, player_id)?;

            repo.bankrupt(conn, player_id)?;
        }
    }

    Ok(())
}


/// 이동 + 월급만 선반영 (구매 결정 대기 시 사용)
pub fn pre_apply_move_salary(
    conn: &Connection,
    player_id: i32,
    new_position: i32,
    new_lap: i32,
    salary: i32,
) -> rusqlite::Result<()> {
    update_position_and_lap(conn, player_id, new_position, new_lap)?;
    if salary > 0 {
        update_money(conn, player_id, salary)?;
        record_transaction(conn, player_id, "deposit", salary, "salary")?;
    }
    Ok(())
}

/// 구매 확정 시 DB 반영 (출금 + 거래기록 + 소유권 설정)
pub fn apply_purchase(
    conn: &Connection,
    player_id: i32,
    tile_position: i32,
    tile_price: i32,
) -> rusqlite::Result<()> {
    update_money(conn, player_id, -tile_price)?;
    record_transaction(
        conn,
        player_id,
        "withdraw",
        tile_price,
        &format!("tile{}_purchase", tile_position),
    )?;
    set_owner(conn, tile_position, player_id, tile_price)?;
    Ok(())
}