//! # 턴 핸들러 (Turn Handler)
//!
//! 6개의 얇은 HTTP 엔드포인트 함수만 제공.
//! 비즈니스 로직은 service::game_service 에 위임한다.

use actix_web::{get, post, web, HttpResponse};

use crate::AppState;
use crate::service::game_service;

// 타입 재공개 (main.rs의 AppState, 테스트 등에서 사용)
pub use game_service::{SessionState, PendingTurn};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  요청 바디 DTO
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(serde::Deserialize)]
pub struct DecideBody {
    pub will_buy: bool,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  6개 API 핸들러 (HTTP 입출력 전용)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[get("/api/state")]
pub async fn get_state(data: web::Data<AppState>) -> HttpResponse {
    let session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };
    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match game_service::get_state(&conn, &session) {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[post("/api/turn")]
pub async fn post_turn(data: web::Data<AppState>) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };

    if session.game_finished {
        return HttpResponse::Conflict().body("게임이 이미 종료되었습니다.");
    }

    if session.pending.is_some() {
        return HttpResponse::Conflict().body("이전 턴의 구매 결정을 먼저 완료해주세요.");
    }

    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match game_service::process_turn(&conn, &mut session) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[post("/api/decide")]
pub async fn post_decide(
    body: web::Json<DecideBody>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };

    if session.pending.is_none() {
        return HttpResponse::BadRequest().body("대기 중인 구매 결정이 없습니다.");
    }

    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match game_service::process_decide(&conn, &mut session, body.will_buy) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[get("/api/transactions/{player_id}")]
pub async fn get_transaction(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match game_service::get_transactions(&conn, path.into_inner()) {
        Ok(txs) => HttpResponse::Ok().json(txs),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[get("/api/result")]
pub async fn get_result(data: web::Data<AppState>) -> HttpResponse {
    let session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };
    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    let result = game_service::get_result(&conn, &session);
    HttpResponse::Ok().json(result)
}

#[post("/api/reset")]
pub async fn post_reset(data: web::Data<AppState>) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };
    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match game_service::reset_game(&conn, &mut session) {
        Ok(_) => HttpResponse::Ok().body("reset success"),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}