/// 타일 도착 시 부동산 관련 행동을 결정하는 함수
/// 소유자 여부에 따라 통행료 지불 or 구매or 파산 등을 판단
pub enum BuyResult {
    PayToll { owner_id: i32, amount: i32 },
    Bankrupt { owner_id: i32, paid: i32 },
    CanBuy { price: i32 },
    Skip,
}

pub fn decide_buy_property(
    player_id: i32,
    money: i32,
    tile_price: i32,
    tile_toll: i32,
    owner: Option<i32>,
    will_buy: bool,
    tile_type: String,
) -> BuyResult {

    if tile_type == "event" {
        return BuyResult::Skip;
    }

    if tile_type == "start" {
        return BuyResult::Skip;
    }

    match owner {
        // 1. 이미 소유자가 있는 경우 → 통행료 처리
        Some(owner_id) => {
            // 소유자가 본인인 경우 skip
            if owner_id == player_id {
                return BuyResult::Skip;
            }
            if money >= tile_toll {
                // 통행료 지불 가능 → 통행료 지불
                BuyResult::PayToll {
                    owner_id,
                    amount: tile_toll,
                }
            } else {
                // 잔액 부족 → 가진 돈 전부 지불 후 파산
                BuyResult::Bankrupt {
                    owner_id,
                    paid: money,
                }
            }
        }

        // 2. 소유자가 없는 경우: 구매 여부 판단
        None => {
            if will_buy {
                if money >= tile_price {
                    // 구매 가능
                    BuyResult::Purchase {
                        price: tile_price,
                    }
                } else {
                    // 잔액 부족 → 구매 불가
                    BuyResult::NotEnoughMoney
                }
            } else {
                // 구매하지 않음
                BuyResult::Skip
            }
        }
    }
}