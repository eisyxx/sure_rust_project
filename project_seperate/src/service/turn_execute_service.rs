use rusqlite::Connection;

use crate::repository::{
    player_repo::{update_money, update_position_and_lap, bankrupt},
    property_repo::{set_owner, reset_owner_for_player},
    transcaction_repo::record_transaction,
};

use crate::service::turn_service::{TurnResult, TurnAction};

pub fn apply_turn_result(
    conn: &Connection,
    player_id: i32,
    result: &TurnResult,
) -> rusqlite::Result<()> {

    // 1. 위치 + lap
    update_position_and_lap(
        conn,
        player_id,
        result.new_position,
        result.new_lap,
    )?;

    // 2. 월급 처리 (입금)
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

    // 3. 액션 처리
    match &result.action {

        // 구매
        TurnAction::Purchase { price } => {
            update_money(conn, player_id, -*price)?;

            record_transaction(
                conn,
                player_id,
                "withdraw",
                *price,
                "tile_purchase",
            )?;

            set_owner(
                conn,
                result.new_position,
                player_id,
                *price,
            )?;
        }

        // 통행료
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
            // 가진 돈 전부 토지 소유자에게 지급
            update_money(conn, *owner_id, *paid)?;
            record_transaction(
                conn,
                *owner_id,
                "deposit",
                *paid,
                &format!("bankrupt_from_{}", player_id),
            )?;

            // 내 출금 기록
            record_transaction(
                conn,
                player_id,
                "withdraw",
                *paid,
                "bankrupt",
            )?;

            // 소유했던 토지 초기화
            reset_owner_for_player(conn, player_id)?;

            bankrupt(conn, player_id)?;
        }

        TurnAction::None => {}
    }

    Ok(())
}