use rusqlite::Connection;
//use actix_web::{web, App, HttpServer};
use std::sync::Mutex;

mod repository;
mod service;
mod handler;

use repository::init::init_db::init_db;
use handler::turn_handler::handle_turn;

fn main() -> rusqlite::Result<()> {
    let conn = Connection::open("game.db")?;
    crate::repository::init::init_db::init_db(&conn)?; // DB 생성 함수

    println!("main init_db 완료");

    let mut current_turn_index = 0;

    // 4. 게임 루프
    loop {
        // handle_turn: 한 턴 진행, true 반환 시 게임 종료
        let game_finished = handle_turn(&conn, &mut current_turn_index)?;

        if game_finished {
            println!("게임 종료!");
            break;
        }

        // 잠깐 대기: 콘솔에서 보기 좋게 하기 (선택)
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}

/* 
// 공유 상태
pub struct AppState {
    pub conn: Mutex<Connection>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let conn = Connection::open("game.db").expect("DB 열기 실패");

    //DB 초기화
    init_db(&conn).expect("DB 초기화 실패");

    println!("게임 서버 실행!");

    let app_state = web::Data::new(AppState {
        conn: Mutex::new(conn),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/api/turn", web::post().to(turn_api))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

*/