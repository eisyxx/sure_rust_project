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

#[derive(Serialize)]
pub struct PlayerState {
    pub id: i64,
    pub name: String,
    pub position: i32,
    pub lap: i32,
    pub money: i32,
    pub turn_order: i32,
}

#[derive(Serialize)]
pub struct GameState {
    pub players: Vec<PlayerState>,
    pub current_player_id: i64,
}

#[derive(Serialize)]
pub struct TransactionItem {
    pub id: i64,
    pub tx_type: String,
    pub amount: i32,
    pub target: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct TransactionHistoryResponse {
    pub player_id: i64,
    pub player_name: String,
    pub transactions: Vec<TransactionItem>,
}