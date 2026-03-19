// src/db/init.rs

use rusqlite::{Connection, Result};

pub fn init_db(conn: &Connection) -> Result<()> {
    
    // 1. games: 게임 전체 상태 관리
    // id=게임 ID(1로 고정? 혹은 제거), current_turn=현재 턴 플레이어의 turn_order, status=게임 상태(playing, finished)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS games (
            current_turn INTEGER,
            status TEXT
        )",
        [],
    )?;


    // 2. players: 플레이어 상태 저장
    // id=플레이어 id, game_id=어떤 게임에 속했는지(현재 1로 고정), position=현재 위치(칸 번호), money=잔액, lap=몇 바퀴 돌았는지, turn_over=턴 순서, is_bankrupt=(0 정상 1 파산)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS players (
            id INTEGER PRIMARY KEY,
            game_id INTEGER,
            name TEXT,
            position INTEGER,
            money INTEGER,
            lap INTEGER,
            turn_order INTEGER,
            is_bankrupt INTEGER
        )",
        [],
    )?;


    // 3. tiles: 맵 정보
    // id=칸 번호, name=칸 이름, type=칸 종류(property, start, eventA~C(사회복지, 세금, 기금)), price=땅 가격
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tiles (
            id INTEGER PRIMARY KEY,
            name TEXT,
            type TEXT,
            price INTEGER,
            toll INTEGER
        )",
        [],
    )?;

    // 4. properties: 토지 소유 정보
    // tile_id=토지 id(tiles.id 연결), owner_id=소유자 (NULL이면 소유자 없음), price=토지 가격
    conn.execute(
        "CREATE TABLE IF NOT EXISTS properties (
            tile_id INTEGER PRIMARY KEY,
            owner_id INTEGER,
            price INTEGER
        )",
        [],
    )?;

    // 5. transactions: 거래 내역
    // id=거래 id, player_id=플레이어 id, type=(deposit 입금, withdraw 출금), amount=금액, target=거래 대상, created_at=거래 시각
    conn.execute(
        "CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY,
            player_id INTEGER,
            type TEXT,
            amount INTEGER,
            target TEXT,
            created_at TEXT
        )",
        [],
    )?;

    // 6. event_tiles: 이벤트 정보
    // tile_id: 토지 번호(tile.id 연결), event_type=(A 사회복지기금, B 종합부동산세, 기금 수령처), amount=금액, description=이벤트 설명(UI용)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_tiles (
            tile_id INTEGER PRIMARY KEY,
            event_type TEXT,
            amount INTEGER,
            description TEXT
        )",
        [],
    )?;

    // 7. fund: 사회복지기금 정보
    // amount=잔액
    conn.execute( 
        "CREATE TABLE IF NOT EXISTS fund ( 
        amount INTEGER )",
        [],
    )?;

    println!("DB 생성 완료");
    Ok(())
}