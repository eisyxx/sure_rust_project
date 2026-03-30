use rusqlite::Connection;
use serde::Serialize;
use crate::repository::player_repo::{get_player_states, PlayerState};

use crate::service::turn_service_all::{
    execute_turn,
    SessionState,
    TurnInput,
    TurnResult,
};

#[derive(Serialize)]
pub struct GameStateResponse {
    pub players: Vec<PlayerState>,
    pub session: SessionState,
}

pub fn handle_turn_api(
    conn: &Connection,
    session: &mut SessionState,
) -> Result<TurnResult, String> {

    execute_turn(
        conn,
        session,
        TurnInput::RollDice,
    )
}

pub fn handle_decide_api(
    conn: &Connection,
    session: &mut SessionState,
    will_buy: bool,
) -> Result<TurnResult, String> {

    execute_turn(
        conn,
        session,
        TurnInput::Decide { will_buy },
    )
}

pub fn handle_end_turn_api(
    conn: &Connection,
    session: &mut SessionState,
) -> Result<TurnResult, String> {

    execute_turn(
        conn,
        session,
        TurnInput::EndTurn,
    )
}

pub fn get_state(
    conn: &Connection,
    session: &SessionState,
) -> Result<GameStateResponse, String> {

    let players = get_player_states(conn)
        .map_err(|e| e.to_string())?;

    Ok(GameStateResponse {
        players,
        session: session.clone(),
    })
}