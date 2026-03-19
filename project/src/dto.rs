use serde::Serialize;

#[derive(Serialize)]
pub struct TurnResponse {
    pub player_id: i64,
    pub dice: u8,
    pub old_position: i32,
    pub new_position: i32,
    pub passed_start: bool,
    pub game_end: bool,
}