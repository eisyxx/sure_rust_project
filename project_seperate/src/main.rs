#![feature(coverage_attribute)]
#![coverage(off)]

use actix_files::{Files, NamedFile};
use actix_web::{get, web, App, HttpServer};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

use project::handler;
use project::repository;
use project::service::orchestrator;
use project::AppState;

fn frontend_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path)
}

// 메인 페이지
#[get("/")]
async fn index(data: web::Data<AppState>) -> actix_web::Result<NamedFile> {
    let conn = match data.conn.lock() {
        Ok(c) => c,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("DB 잠금 실패")),
    };

    let new_session = orchestrator::init_session(&conn)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let mut session = match data.session.lock() {
        Ok(s) => s,
        Err(_) => return Err(actix_web::error::ErrorInternalServerError("세션 잠금 실패")),
    };

    *session = new_session;

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

// 게임 결과 페이지
#[get("/result")]
async fn result_page() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(frontend_path("result.html"))?)
}

// 서버 실행
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conn = Connection::open("game.db").expect("DB 열기 실패");
    repository::init::init_db::init_db(&conn).expect("DB 초기화 실패");

    println!("게임 서버 실행!");

    let app_state = web::Data::new(AppState {
        conn: Mutex::new(conn),
        session: Mutex::new(orchestrator::SessionState {
            current_turn_index: 0,
            game_finished: false,
            winner_id: None,
            pending: None,
            final_rankings: None,
            players: vec![],
        }),
    });

    let port = std::env::var("PORT").unwrap_or("8080".to_string());

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(index)
            .service(map_script)
            .service(stylesheet)
            .service(result_page)
            .service(handler::get_state)
            .service(handler::post_turn)
            .service(handler::post_decide)
            .service(handler::get_transaction)
            .service(handler::get_result)
            .service(handler::post_reset)
            .service(Files::new("/assets", frontend_path("assets")))
            .service(Files::new("/", frontend_path("")).index_file("index.html"))
    })
    // .bind("127.0.0.1:8080")? // 로컬에서 테스트할 때 사용
    .bind(format!("0.0.0.0:{}", port))? // 클라우드에서 실행할 때 사용
    .run()
    .await
}