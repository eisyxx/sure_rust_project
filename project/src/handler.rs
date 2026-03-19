use actix_web::{post, get, web, HttpResponse, Responder};
use serde::Serialize;
use std::sync::Mutex;
use rusqlite::Connection;

use crate::dto::TurnResponse;
use crate::service::{play_turn_api, next_turn, get_game_state};


// 🎮 게임 상태 조회
#[get("/api/game-state")]
pub async fn game_state_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    match get_game_state(&conn, 1) {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(e) => {
            println!("❌ game_state error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// 🎲 play-turn API
#[post("/api/play-turn")]
pub async fn play_turn_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    match play_turn_api(&conn, 1) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            println!("❌ play_turn error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}


// 🔄 next-turn API
#[derive(Serialize)]
struct SimpleResponse {
    ok: bool,
}

#[post("/api/next-turn")]
pub async fn next_turn_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    match next_turn(&conn, 1) {
        Ok(_) => HttpResponse::Ok().json(SimpleResponse { ok: true }),
        Err(e) => {
            println!("❌ next_turn error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}