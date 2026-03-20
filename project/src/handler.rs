use actix_web::{post, get, web, HttpResponse, Responder};
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use rusqlite::Connection;

use crate::service::{play_turn_api, next_turn, get_game_state, reset_game, get_current_player_transactions, handle_property, PropertyResult, buy_property, distribute_rewards};


// 🔁 게임 초기화
#[post("/api/reset-game")]
pub async fn reset_game_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    match reset_game(&conn, 1).and_then(|_| get_game_state(&conn, 1)) {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(e) => {
            println!("❌ reset_game error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}


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

#[get("/api/transactions/current")]
pub async fn current_transactions_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    match get_current_player_transactions(&conn, 1) {
        Ok(history) => HttpResponse::Ok().json(history),
        Err(e) => {
            println!("❌ current_transactions error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// 토지 소유자 조회
#[derive(Serialize)]
pub struct PropertyResponse {
    pub result: String,
}

#[post("/api/handle-property")]
pub async fn handle_property_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    // 현재 플레이어 가져오기
    let player_id: i64 = match conn.query_row(
        "SELECT id FROM players WHERE current_turn = 1",
        [],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(e) => {
            println!("❌ get current player error: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    match handle_property(&conn, player_id) {
    Ok(result) => {
        let res_str = match result {
            PropertyResult::TollPaid => "TOLL",
            PropertyResult::CanBuy => "CAN_BUY",
            PropertyResult::Nothing => "NONE",
        };

        HttpResponse::Ok().json(PropertyResponse {
            result: res_str.to_string(),
        })
    },
        Err(e) => {
            println!("❌ handle_property error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}


// 토지 구매 (버튼용)
#[derive(Deserialize)]
pub struct BuyRequest {
    pub tile_id: i32,
}

#[post("/api/buy-property")]
pub async fn buy_property_handler(
    db: web::Data<Mutex<Connection>>,
    req: web::Json<BuyRequest>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    let player_id: i64 = match conn.query_row(
        "SELECT id FROM players WHERE current_turn = 1",
        [],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(e) => {
            println!("❌ get current player error: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // tile 가격 조회
    let price: i32 = match conn.query_row(
        "SELECT price FROM tiles WHERE id = ?1",
        (req.tile_id,),
        |row| row.get(0),
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("❌ tile 조회 실패: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    match buy_property(&conn, player_id, req.tile_id, price) {
        Ok(_) => HttpResponse::Ok().json(SimpleResponse { ok: true }),
        Err(e) => {
            println!("❌ buy_property error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// 상금 지급 및 승자 발표
#[derive(Serialize)]
pub struct RewardResponse {
    pub winner_id: i64,
}

#[post("/api/distribute-rewards")]
pub async fn distribute_rewards_handler(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();

    match distribute_rewards(&conn) {
        Ok(winner_id) => HttpResponse::Ok().json(RewardResponse { winner_id }),
        Err(e) => {
            println!("❌ distribute_rewards error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}