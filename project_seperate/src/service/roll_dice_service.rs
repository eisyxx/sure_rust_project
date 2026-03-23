use rand::Rng;

// 주사위: 1~6 사이의 값을 랜덤하게 반환
pub fn roll_dice() -> i32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=6)
}

/* 테스트 가능 버전
pub fn roll_dice_with_rng<R: rand::Rng>(rng: &mut R) -> i32 {
    rng.gen_range(1..=6)
}
*/