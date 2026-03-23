use rusqlite::Connection;

use crate::repository::{
    property_repo::get_player_total_property_price,
    tile_repo::get_event_tile_info,
};

pub enum EventResult {
    EstateTax { amount: i32 },
    EstateTaxSkipped,
    None,
}

pub fn handle_event(conn: &Connection, player_id: i32, tile_id: i32) -> EventResult {
    let (event_type, amount) = match get_event_tile_info(conn, tile_id) {
        Ok(Some(info)) => info,
        _ => return EventResult::None,
    };

    match event_type.as_str() {
        "tax_if_property" => {
            let total = get_player_total_property_price(conn, player_id).unwrap_or(0);
            if total >= 100 {
                EventResult::EstateTax { amount }
            } else {
                EventResult::EstateTaxSkipped
            }
        }
        _ => EventResult::None,
    }
}