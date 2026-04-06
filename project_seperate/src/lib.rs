use rusqlite::Connection;
use std::sync::Mutex;

pub mod service;
pub mod handler;
pub mod repository;

use crate::service::orchestrator;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub session: Mutex<orchestrator::SessionState>,
}

#[cfg(test)]
mod unit_test;