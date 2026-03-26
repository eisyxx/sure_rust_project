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
