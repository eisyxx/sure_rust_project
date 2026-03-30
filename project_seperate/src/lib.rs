use rusqlite::Connection;
use std::sync::Mutex;

pub mod service;
pub mod handler;
pub mod repository;

use crate::service::game_service;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub session: Mutex<game_service::SessionState>,
}

#[cfg(test)]
mod unit_test;