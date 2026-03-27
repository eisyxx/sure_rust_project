use actix_files::{Files, NamedFile};
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::Serialize;


mod repository;
mod service;
mod handler;
use crate::service::game_end_service::Player as GamePlayer;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub session: Mutex<handler::turn_handler::SessionState>,
}

fn frontend_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path)
}

// 메인 페이지
#[get("/")]
async fn index(data: web::Data<AppState>) -> actix_web::Result<NamedFile> {
    let conn = match data.conn.lock() {
        Ok(conn) => conn,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("DB 잠금 실패")),
    };

    repository::init::init_db::init_db(&conn)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // DB에서 모든 플레이어 가져오기
    let db_players = repository::player_repo::get_all_players(&conn)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // GamePlayer로 변환
    let game_players: Vec<GamePlayer> = db_players
        .into_iter()
        .map(|p| GamePlayer {
            id: p.id,
            position: p.position,
            money: p.money,
            lap: p.lap,
            is_bankrupt: p.is_bankrupt,
        })
        .collect();

    //세션 초기화
    let mut session = match data.session.lock() {
        Ok(session) => session,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("세션 잠금 실패")),
    };

    *session = handler::turn_handler::SessionState {
        current_turn_index: 0,
        game_finished: false,
        winner_id: None,
        pending: None,
        final_rankings: None,
        players: game_players,
    };

    Ok(NamedFile::open(frontend_path("index.html"))?)
}

// JS 파일
#[get("/map.js")]
async fn map_script() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(frontend_path("map.js"))?)
}

// CSS 파일
#[get("/style.css")]
async fn stylesheet() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(frontend_path("style.css"))?)
}

// 현재 게임 상태 API
#[get("/api/state")]
async fn game_state(data: web::Data<AppState>) -> HttpResponse {
    let session = match data.session.lock() {
        Ok(session) => session,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };
    let conn = match data.conn.lock() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match handler::turn_handler::get_state(&conn, &session) {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

// 턴 진행 API
#[post("/api/turn")]
async fn turn_api(data: web::Data<AppState>) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(session) => session,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };

    if session.game_finished {
        return HttpResponse::Conflict().body("게임이 이미 종료되었습니다.");
    }

    if session.pending.is_some() {
        return HttpResponse::Conflict().body("이전 턴의 구매 결정을 먼저 완료해주세요.");
    }

    let conn = match data.conn.lock() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match handler::turn_handler::handle_turn(&conn, &mut session) {
        Ok(turn_result) => HttpResponse::Ok().json(turn_result),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

// 구매 결정 API
#[derive(serde::Deserialize)]
struct DecideBody {
    will_buy: bool,
}

#[post("/api/decide")]
async fn decide_api(body: web::Json<DecideBody>, data: web::Data<AppState>) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(session) => session,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };

    if session.pending.is_none() {
        return HttpResponse::BadRequest().body("대기 중인 구매 결정이 없습니다.");
    }

    let conn = match data.conn.lock() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match handler::turn_handler::handle_decide(&conn, &mut session, body.will_buy) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

// 특정 플레이어 거래 내역 조회 API
#[get("/api/transactions/{player_id}")]
async fn player_transactions(path: web::Path<i32>, data: web::Data<AppState>) -> HttpResponse {
    let conn = match data.conn.lock() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    match handler::turn_handler::get_transactions(&conn, path.into_inner()) {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

// 게임 결과 라우터
#[get("/result")]
async fn result_page() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(frontend_path("result.html"))?)
}

// 게임 결과
#[derive(Serialize)]
struct PlayerFrontend {
    id: i32,
    name: String,
    image_url: String,
    money: i32,
    is_bankrupt: bool,
    rank: Option<usize>,
}
fn get_frontend_players(
    conn: &Connection,
    final_rankings: &Option<Vec<(i32, i32)>>
) -> Vec<PlayerFrontend> {
    let all_players = match repository::player_repo::get_all_players(conn) {
        Ok(players) => players,
        Err(_) => return vec![], // DB 오류 시 빈 배열 반환
    };

    if let Some(rankings) = final_rankings {
        rankings
            .iter()
            .enumerate()
            .map(|(i, (player_id, money))| {

                let player_opt = all_players.iter().find(|p| p.id == *player_id);

                match player_opt {
                    Some(p) => PlayerFrontend {
                        id: p.id,
                        name: format!("Player {}", p.id),

                        image_url: format!("/assets/player{}_icon.png", p.id),

                        money: *money,
                        is_bankrupt: p.is_bankrupt,
                        rank: if p.is_bankrupt { None } else { Some(i + 1) },
                    },
                    None => {
                        println!("⚠️ player_id {} not found in DB", player_id);

                        PlayerFrontend {
                            id: *player_id,
                            name: format!("Player {}", player_id),
                            image_url: format!("/assets/player{}_icon.png", player_id),
                            money: *money,
                            is_bankrupt: true,
                            rank: None,
                        }
                    }
                }
            })
            .collect()
    } else {
        // 게임 아직 안 끝난 경우
        all_players
            .iter()
            .map(|p| PlayerFrontend {
                id: p.id,
                name: format!("Player {}", p.id),
                image_url: format!("/assets/player{}_icon.png", p.id),
                money: p.money,
                is_bankrupt: p.is_bankrupt,
                rank: None,
            })
            .collect()
    }
}

#[get("/api/result")]
async fn game_result(data: web::Data<AppState>) -> HttpResponse {
    let session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };
    let conn = match data.conn.lock() {
            Ok(c) => c,
            Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
        };

    let frontend_players = get_frontend_players(&conn, &session.final_rankings);

    HttpResponse::Ok().json(frontend_players)
}

// 게임 재시작
#[post("/api/reset")]
async fn reset_game(data: web::Data<AppState>) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };

    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB 잠금 실패"),
    };

    // DB 초기화
    if let Err(e) = repository::init::init_db::init_db(&conn) {
        return HttpResponse::InternalServerError().body(e.to_string());
    }

    // 세션 초기화
    session.current_turn_index = 0;
    session.game_finished = false;
    session.winner_id = None;
    session.pending = None;
    session.final_rankings = None;

    HttpResponse::Ok().body("reset success")
}

// 서버 실행
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // DB 열기 및 초기화
    let conn = Connection::open("game.db").expect("DB 열기 실패");
    repository::init::init_db::init_db(&conn).expect("DB 초기화 실패");

    println!("게임 서버 실행!");

    let app_state = web::Data::new(AppState {
        conn: Mutex::new(conn),
        session: Mutex::new(handler::turn_handler::SessionState {
            current_turn_index: 0,
            game_finished: false,
            winner_id: None,
            pending: None,
            final_rankings: None,
            players: vec![],
        }),
    });

    //HTTP 서버 설정
    let port = std::env::var("PORT").unwrap_or("8080".to_string());

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(index)
            .service(map_script)
            .service(stylesheet)
            .service(game_state)
            .service(result_page)
            .service(reset_game)
            .service(turn_api)
            .service(decide_api)
            .service(player_transactions)
            .service(game_result) 
                .service(Files::new("/assets", frontend_path("assets")))
            .service(Files::new("/", frontend_path("")).index_file("index.html"))
    })
    // .bind(format!("0.0.0.0:{}", port))?
    .bind("127.0.0.1:8080")?
    .run()
    .await
}