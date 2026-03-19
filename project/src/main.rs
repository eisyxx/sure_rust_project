use actix_web::{App, HttpServer, web};
use std::sync::Mutex;
use rusqlite::{Connection, Result};

mod init_db;
mod handler;
mod service;
mod dto;

use init_db::create_db::init_db;
use init_db::init_player::create_player;
use init_db::init_tiles::init_tiles;

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
            .service(handler::game_state_handler)
            .service(handler::play_turn_handler)
            .service(handler::next_turn_handler)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

fn init() -> Result<()> {
    let conn = Connection::open("game.db")?;

    init_db(&conn)?;

    // (개발 중) 초기화
    conn.execute("DELETE FROM players", [])?;
    conn.execute("DELETE FROM games", [])?;
    conn.execute("DELETE FROM tiles", [])?;
    conn.execute("UPDATE players SET is_bankrupt = 0",[],)?;

    conn.execute(
        "INSERT INTO games(current_turn, status)
         VALUES (1,'playing')",
        [],
    )?;

    init_tiles(&conn)?;

    create_player(&conn, 1, "Player1", 1)?;
    create_player(&conn, 1, "Player2", 2)?;
    create_player(&conn, 1, "Player3", 3)?;
    create_player(&conn, 1, "Player4", 4)?;

    // current_turn 안전 세팅
    conn.execute("UPDATE players SET current_turn = 0", [])?;

    conn.execute(
        "UPDATE players
         SET current_turn = 1
         WHERE turn_order = 1",
        [],
    )?;

    Ok(())
}