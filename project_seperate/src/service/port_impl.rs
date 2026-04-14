use rusqlite::Connection;

use crate::repository::{
    event_repo::{add_fund, get_event_info, get_fund_amount, reset_fund},
    player_repo::{bankrupt, get_player_money, get_player_states, update_money, update_position_and_lap, PlayerState},
    property_repo::{get_player_total_property_price, reset_owner_for_player},
    transcaction_repo::record_transaction,
};

use crate::service::{
    event_service::{handle_event, EventResult},
    roll_dice_service::roll_dice,
    traits::{EventServiceRepo, PlayerStateRepo, TurnExecuteRepo, TurnServiceDeps},
};

pub struct PortImpl;

impl EventServiceRepo for PortImpl {
    fn get_event_info(&self, conn: &Connection, tile_id: i32) -> rusqlite::Result<(String, i32)> {
        get_event_info(conn, tile_id)
    }

    fn get_player_money(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<i32> {
        get_player_money(conn, player_id)
    }

    fn get_player_total_property_price(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<i32> {
        get_player_total_property_price(conn, player_id)
    }

    fn get_fund_amount(&self, conn: &Connection) -> rusqlite::Result<i32> {
        get_fund_amount(conn)
    }
}

impl TurnServiceDeps for PortImpl {
    fn roll_dice(&self) -> i32 {
        roll_dice()
    }

    fn handle_event(&self, conn: &Connection, player_id: i32, tile_id: i32) -> EventResult {
        handle_event(conn, player_id, tile_id)
    }
}

impl PlayerStateRepo for PortImpl {
    fn get_player_states(&self, conn: &Connection) -> rusqlite::Result<Vec<PlayerState>> {
        get_player_states(conn)
    }
}

impl TurnExecuteRepo for PortImpl {
    fn update_position_and_lap(&self, conn: &Connection, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()> {
        update_position_and_lap(conn, player_id, pos, lap)
    }

    fn update_money(&self, conn: &Connection, player_id: i32, delta: i32) -> rusqlite::Result<()> {
        update_money(conn, player_id, delta)
    }

    fn record_transaction(&self, conn: &Connection, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()> {
        record_transaction(conn, player_id, tx_type, amount, target)
    }

    fn reset_owner_for_player(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<()> {
        reset_owner_for_player(conn, player_id)
    }

    fn bankrupt(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<()> {
        bankrupt(conn, player_id)
    }

    fn add_fund(&self, conn: &Connection, amount: i32) -> rusqlite::Result<()> {
        add_fund(conn, amount)
    }

    fn reset_fund(&self, conn: &Connection) -> rusqlite::Result<()> {
        reset_fund(conn)
    }
}
