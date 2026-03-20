use actix_files::{Files, NamedFile};
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

mod repository;
mod service;
mod handler;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub session: Mutex<handler::turn_handler::SessionState>,
}

fn frontend_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path)
}

#[get("/")]
async fn index(data: web::Data<AppState>) -> actix_web::Result<NamedFile> {
    let conn = match data.conn.lock() {
        Ok(conn) => conn,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("DB 잠금 실패")),
    };

    repository::init::init_db::init_db(&conn)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let mut session = match data.session.lock() {
        Ok(session) => session,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("세션 잠금 실패")),
    };

    *session = handler::turn_handler::SessionState {
        current_turn_index: 0,
        game_finished: false,
        winner_id: None,
        pending: None,
    };

    Ok(NamedFile::open(frontend_path("index.html"))?)
}

#[get("/map.js")]
async fn map_script() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(frontend_path("map.js"))?)
}

#[get("/style.css")]
async fn stylesheet() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(frontend_path("style.css"))?)
}

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

#[post("/api/turn")]
async fn turn_api(data: web::Data<AppState>) -> HttpResponse {
    let mut session = match data.session.lock() {
        Ok(session) => session,
        Err(_) => return HttpResponse::InternalServerError().body("세션 잠금 실패"),
    };

    if session.game_finished {
        return HttpResponse::Conflict().body("게임이 이미 종료되었습니다.");
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
        }),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(index)
            .service(map_script)
            .service(stylesheet)
            .service(game_state)
            .service(turn_api)
            .service(decide_api)
            .service(player_transactions)
                .service(Files::new("/assets", frontend_path("assets")))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}