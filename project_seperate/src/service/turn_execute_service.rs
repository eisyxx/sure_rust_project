use rusqlite::Connection;

use crate::repository::{
    event_repo,
    player_repo,
    property_repo,
    transcaction_repo,
};

use crate::service::turn_service::{TurnResult, TurnAction};

pub trait TurnExecuteRepository {
    fn update_position_and_lap(&self, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()>;
    fn update_money(&self, player_id: i32, delta: i32) -> rusqlite::Result<()>;
    fn bankrupt(&self, player_id: i32) -> rusqlite::Result<()>;
    fn set_owner(&self, tile_id: i32, owner_id: i32, price: i32) -> rusqlite::Result<()>;
    fn reset_owner_for_player(&self, player_id: i32) -> rusqlite::Result<()>;
    fn record_transaction(&self, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()>;
    fn add_fund(&self, amount: i32) -> rusqlite::Result<()>;
    fn reset_fund(&self) -> rusqlite::Result<()>;
}

pub struct DbTurnExecuteRepository<'a> {
    conn: &'a Connection,
}

impl<'a> DbTurnExecuteRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> TurnExecuteRepository for DbTurnExecuteRepository<'a> {
    fn update_position_and_lap(&self, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()> {
        player_repo::update_position_and_lap(self.conn, player_id, pos, lap)
    }

    fn update_money(&self, player_id: i32, delta: i32) -> rusqlite::Result<()> {
        player_repo::update_money(self.conn, player_id, delta)
    }

    fn bankrupt(&self, player_id: i32) -> rusqlite::Result<()> {
        player_repo::bankrupt(self.conn, player_id)
    }

    fn set_owner(&self, tile_id: i32, owner_id: i32, price: i32) -> rusqlite::Result<()> {
        property_repo::set_owner(self.conn, tile_id, owner_id, price)
    }

    fn reset_owner_for_player(&self, player_id: i32) -> rusqlite::Result<()> {
        property_repo::reset_owner_for_player(self.conn, player_id)
    }

    fn record_transaction(&self, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()> {
        transcaction_repo::record_transaction(self.conn, player_id, tx_type, amount, target)
    }

    fn add_fund(&self, amount: i32) -> rusqlite::Result<()> {
        event_repo::add_fund(self.conn, amount)
    }

    fn reset_fund(&self) -> rusqlite::Result<()> {
        event_repo::reset_fund(self.conn)
    }
}

// process_turn 함수 실행 결과를 DB에 반영하는 함수
pub fn apply_turn_result_with_repo<R: TurnExecuteRepository>(
    repo: &R,
    player_id: i32,
    result: &TurnResult,
) -> rusqlite::Result<()> {

    // 위치 + lap(바퀴 수) 업데이트
    repo.update_position_and_lap(
        player_id,
        result.new_position,
        result.new_lap,
    )?;

    // 월급 처리 (입금 후 내역 기록)
    if result.salary > 0 {
        repo.update_money(player_id, result.salary)?;

        repo.record_transaction(
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
            repo.update_money(player_id, -*price)?;

            repo.record_transaction(
                player_id,
                "withdraw",
                *price,
                &format!("tile{}_purchase", result.new_position),
            )?;

            repo.set_owner(
                result.new_position,
                player_id,
                *price,
            )?;
        }

        // 통행료 지급
        TurnAction::PayToll { owner_id, amount } => {
            // 잔액 출금
            repo.update_money(player_id, -*amount)?;
            repo.record_transaction(
                player_id,
                "withdraw",
                *amount,
                &format!("toll_to_{}", owner_id),
            )?;

            // 토지 소유자 잔액 증가
            repo.update_money(*owner_id, *amount)?;
            repo.record_transaction(
                *owner_id,
                "deposit",
                *amount,
                &format!("toll_from_{}", player_id),
            )?;
        }

        // 파산
        TurnAction::Bankrupt { owner_id, paid } => {
            // 파산 플레이어 잔액을 먼저 차감해야 거래 전/후 잔액이 올바르게 기록된다.
            repo.update_money(player_id, -*paid)?;

            // 잔액을 전부 토지 소유자에게 지급
            repo.update_money(*owner_id, *paid)?;
            repo.record_transaction(
                *owner_id,
                "deposit",
                *paid,
                &format!("bankrupt_from_{}", player_id),
            )?;

            // 출금 내역 기록
            repo.record_transaction(
                player_id,
                "withdraw",
                *paid,
                &format!("bankrupt_to_{}", owner_id),
            )?;

            // 소유했던 토지 초기화
            repo.reset_owner_for_player(player_id)?;

            // 파산 처리
            repo.bankrupt(player_id)?;
        }

        // 이벤트 A: 사회복지기금
        TurnAction::EventWelfareFund { amount } => {
            // 돈 차감
            repo.update_money(player_id, -*amount)?;

            // 기금 증가
            repo.add_fund(*amount)?;

            // 거래 기록
            repo.record_transaction(
                player_id,
                "withdraw",
                *amount,
                "welfare_fund",
            )?;
        }

        // 이벤트 A: 파산
        TurnAction::EventWelfareFundBankrupt { paid } => {
            // 가진 돈 전부 기금으로
            repo.add_fund(*paid)?;

            // 거래 기록
            repo.record_transaction(
                player_id,
                "withdraw",
                *paid,
                "welfare_fund_bankrupt",
            )?;

            // 토지 초기화
            repo.reset_owner_for_player(player_id)?;

            // 파산 처리
            repo.bankrupt(player_id)?;
        }

        // 이벤트 C: 기금 없음
        TurnAction::FundReceiveEmpty => {}

        // 이벤트 C: 기금 수령
        TurnAction::EventFundReceive { amount } => {
            // 플레이어 돈 증가
            repo.update_money(player_id, *amount)?;

            // 거래 기록
            repo.record_transaction(
                player_id,
                "deposit",
                *amount,
                "welfare_fund_receive",
            )?;

            // 기금 초기화
            repo.reset_fund()?;
        }

        TurnAction::None => {}

        TurnAction::EstateTaxSkipped => {}

        // 이벤트 B: 종합부동산세
        TurnAction::EstateTax { amount } => {
            // 플레이어 돈 차감
            repo.update_money(player_id, -*amount)?;

            // 거래 기록
            repo.record_transaction(
                player_id,
                "withdraw",
                *amount,
                "estate_tax",
            )?;
        }

        // 이벤트 B: 파산
        TurnAction::EstateTaxBankrupt { paid } => {
            // 가진 돈 전부 차감
            repo.update_money(player_id, -*paid)?;

            repo.record_transaction(
                player_id,
                "withdraw",
                *paid,
                "estate_tax_bankrupt",
            )?;

            // 토지 초기화
            repo.reset_owner_for_player(player_id)?;

            // 파산 처리
            repo.bankrupt(player_id)?;
        }
    }

    Ok(())
}

// Connection 기반 wrapper (기존 호출부 호환)
pub fn apply_turn_result(
    conn: &Connection,
    player_id: i32,
    result: &TurnResult,
) -> rusqlite::Result<()> {
    let repo = DbTurnExecuteRepository::new(conn);
    apply_turn_result_with_repo(&repo, player_id, result)
}