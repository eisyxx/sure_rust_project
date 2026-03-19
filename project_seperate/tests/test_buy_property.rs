use crate::service::buy_property_service;

#[test]
fn test_pay_toll() {
    let result = decide_buy_property(1000, 500, 200, Some(2), false);

    match result {
        BuyResult::PayToll { amount, .. } => assert_eq!(amount, 200),
        _ => panic!("wrong result"),
    }
}