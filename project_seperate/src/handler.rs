//! # 턴 핸들러 (Turn Handler)
//!
//! 6개의 HTTP 엔드포인트 함수 + DTO 조립을 담당.
//! 비즈니스 로직은 service::game_service 에 위임한다.

use actix_web::{get, post, web, HttpResponse};
use serde::Serialize;

use crate::AppState;
use crate::service::orchestrator;
use crate::repository::player_repo::PlayerState;
use crate::repository::property_repo::TileOwnerRecord;
use crate::repository::transcaction_repo::TransactionRecord;

// 타입 재공개 (main.rs의 AppState, 테스트 등에서 사용)
pub use orchestrator::{SessionState, PendingTurn};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  요청/응답 DTO
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(serde::Deserialize)]
pub struct DecideBody {
    pub will_buy: bool,
}

#[derive(Serialize)]
struct ApiPlayer {
    id: i32,
    name: String,
    position: i32,
    money: i32,
    lap: i32,
    turn_order: i32,
    is_bankrupt: bool,
}

#[derive(Serialize)]
struct ApiTransaction {
    id: i32,
    tx_type: String,
    amount: i32,
    target: String,
    balance_before: i32,
    balance_after: i32,
    created_at: String,
}

#[derive(Serialize)]
struct ApiTileOwner {
    tile_id: i32,
    owner_id: i32,
}

#[derive(Serialize)]
struct ApiStateResponse {
    players: Vec<ApiPlayer>,
    tile_owners: Vec<ApiTileOwner>,
    current_player_id: Option<i32>,
    game_finished: bool,
    winner_id: Option<i32>,
}

#[derive(Serialize)]
struct ApiTurnResponse {
    player_id: i32,
    dice: i32,
    old_position: i32,
    new_position: i32,
    old_lap: i32,
    new_lap: i32,
    salary: i32,
    action_type: &'static str,
    action_amount: i32,
    owner_id: Option<i32>,
    players: Vec<ApiPlayer>,
    tile_owners: Vec<ApiTileOwner>,
    current_player_id: Option<i32>,
    game_finished: bool,
    winner_id: Option<i32>,
}

#[derive(Serialize)]
struct PlayerFrontend {
    id: i32,
    name: String,
    image_url: String,
    money: i32,
    is_bankrupt: bool,
    rank: Option<usize>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  DTO 매핑 함수
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn map_players(players: Vec<PlayerState>) -> Vec<ApiPlayer> {
    players
        .into_iter()
        .map(|p| ApiPlayer {
            id: p.id,
            name: p.name,
            position: p.position,
            money: p.money,
            lap: p.lap,
            turn_order: p.turn_order,
            is_bankrupt: p.is_bankrupt,
        })
        .collect()
}

fn map_tile_owners(owners: Vec<TileOwnerRecord>) -> Vec<ApiTileOwner> {
    owners
        .into_iter()
        .map(|r| ApiTileOwner {
            tile_id: r.tile_id,
            owner_id: r.owner_id,
        })
        .collect()
}

fn map_transactions(txs: Vec<TransactionRecord>) -> Vec<ApiTransaction> {
    txs.into_iter()
        .map(|tx| ApiTransaction {
            id: tx.id,
            tx_type: tx.tx_type,
            amount: tx.amount,
            target: tx.target,
            balance_before: tx.balance_before,
            balance_after: tx.balance_after,
            created_at: tx.created_at,
        })
        .collect()
}

fn map_result_players(players: Vec<orchestrator::ResultPlayer>) -> Vec<PlayerFrontend> {
    players
        .into_iter()
        .map(|p| PlayerFrontend {
            id: p.id,
            name: format!("Player {}", p.id),
            image_url: format!("/assets/player{}_icon.png", p.id),
            money: p.money,
            is_bankrupt: p.is_bankrupt,
            rank: p.rank,
        })
        .collect()
}

fn map_turn_outcome(o: orchestrator::TurnOutcome) -> ApiTurnResponse {
    ApiTurnResponse {
        player_id: o.player_id,
        dice: o.dice,
        old_position: o.old_position,
        new_position: o.new_position,
        old_lap: o.old_lap,
        new_lap: o.new_lap,
        salary: o.salary,
        action_type: o.action_type,
        action_amount: o.action_amount,
        owner_id: o.owner_id,
        players: map_players(o.players),
        tile_owners: map_tile_owners(o.tile_owners),
        current_player_id: o.current_player_id,
        game_finished: o.game_finished,
        winner_id: o.winner_id,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  6개 API 핸들러
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

    match orchestrator::get_state(&conn, &session) {
        Ok(state) => HttpResponse::Ok().json(ApiStateResponse {
            players: map_players(state.players),
            tile_owners: map_tile_owners(state.tile_owners),
            current_player_id: state.current_player_id,
            game_finished: state.game_finished,
            winner_id: state.winner_id,
        }),
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

    match orchestrator::process_turn(&conn, &mut session) {
        Ok(outcome) => HttpResponse::Ok().json(map_turn_outcome(outcome)),
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

    match orchestrator::process_decide(&conn, &mut session, body.will_buy) {
        Ok(outcome) => HttpResponse::Ok().json(map_turn_outcome(outcome)),
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

    match orchestrator::get_transactions(&conn, path.into_inner()) {
        Ok(txs) => HttpResponse::Ok().json(map_transactions(txs)),
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

    let result = orchestrator::get_result(&conn, &session);
    HttpResponse::Ok().json(map_result_players(result))
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

    match orchestrator::reset_game(&conn, &mut session) {
        Ok(_) => HttpResponse::Ok().body("reset success"),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}