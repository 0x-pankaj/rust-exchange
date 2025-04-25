use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub price: Decimal,
    pub quantity: Decimal,
    pub order_id: String,
    pub filled: Decimal,
    pub side: OrderSide,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum OrderSide {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}
