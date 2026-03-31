use rusqlite::Connection;
use crate::service::event_service::EventResult;

pub trait EventServiceRepo {
    fn get_event_info(&self, conn: &Connection, tile_id: i32) -> rusqlite::Result<(String, i32)>;
    fn get_player_money(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<i32>;
    fn get_player_total_property_price(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<i32>;
    fn get_fund_amount(&self, conn: &Connection) -> rusqlite::Result<i32>;
}

pub trait TurnServiceDeps {
    fn roll_dice(&self) -> i32;
    fn handle_event(&self, conn: &Connection, player_id: i32, tile_id: i32) -> EventResult;
}

pub trait PlayerStateRepo {
    fn get_player_states(&self, conn: &Connection) -> rusqlite::Result<Vec<crate::repository::player_repo::PlayerState>>;
}

pub trait TurnExecuteRepo {
    fn update_position_and_lap(&self, conn: &Connection, player_id: i32, pos: i32, lap: i32) -> rusqlite::Result<()>;
    fn update_money(&self, conn: &Connection, player_id: i32, delta: i32) -> rusqlite::Result<()>;
    fn record_transaction(&self, conn: &Connection, player_id: i32, tx_type: &str, amount: i32, target: &str) -> rusqlite::Result<()>;
    fn set_owner(&self, conn: &Connection, tile_id: i32, player_id: i32, price: i32) -> rusqlite::Result<()>;
    fn reset_owner_for_player(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<()>;
    fn bankrupt(&self, conn: &Connection, player_id: i32) -> rusqlite::Result<()>;
    fn add_fund(&self, conn: &Connection, amount: i32) -> rusqlite::Result<()>;
    fn reset_fund(&self, conn: &Connection) -> rusqlite::Result<()>;
}
