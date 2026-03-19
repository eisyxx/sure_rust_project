use actix_web::{App, HttpServer, web};
use std::sync::Mutex;
use rusqlite::{Connection, Result};

mod init_db;
mod handler;
mod service;
mod dto;

use service::reset_game;

use actix_cors::Cors;



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // DB 초기화만 수행
    if let Err(e) = init() {
        println!("초기화 에러: {:?}", e);
    }

    let conn = Connection::open("game.db").unwrap();
    let data = web::Data::new(Mutex::new(conn));

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
            )
            .app_data(data.clone())
                .service(handler::reset_game_handler)
            .service(handler::game_state_handler)
            .service(handler::play_turn_handler)
            .service(handler::next_turn_handler)
                .service(handler::current_transactions_handler)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

fn init() -> Result<()> {
    let conn = Connection::open("game.db")?;

    reset_game(&conn, 1)
}