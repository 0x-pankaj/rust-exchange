use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CREATE_ORDER: &str = "CREATE_ORDER";
pub const CANCEL_ORDER: &str = "CANCEL_ORDER";
pub const ON_RAMP: &str = "ON_RAMP";
pub const GET_OPEN_ORDERS: &str = "GET_OPEN_ORDERS";
pub const GET_DEPTH: &str = "GET_DEPTH";

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageToEngine {
    pub type_: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum MessageFromOrderbook {
    Depth {
        market: String,
        bids: Vec<(String, String)>,
        asks: Vec<(String, String)>,
    },
    OrderPlaced {
        order_id: String,
        executed_qty: f64,
        fills: Vec<Fill>,
    },
    OrderCancelled {
        order_id: String,
        executed_qty: f64,
        remaining_qty: f64,
    },
    OpenOrders(Vec<OpenOrder>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Fill {
    price: String,
    qty: f64,
    trade_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenOrder {
    order_id: String,
    executed_qty: f64,
    price: String,
    quantity: String,
    side: String,
    user_id: String,
}
