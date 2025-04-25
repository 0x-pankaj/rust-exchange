use serde::{Deserialize, Serialize};

pub const CREATE_ORDER: &str = "CREATE_ORDER";
pub const CANCEL_ORDER: &str = "CANCEL_ORDER";
pub const ON_RAMP: &str = "ON_RAMP";
pub const GET_DEPTH: &str = "GET_DEPTH";
pub const GET_OPEN_ORDER: &str = "GET_OPEN_ORDERS";

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum MessageFromApi {
    #[serde(rename = "CREATE_ORDER")]
    CreateOrder {
        data: CreateOrderData,
        client_id: String,
    },

    #[serde(rename = "CANCEL_ORDER")]
    CancelOrder {
        data: CancelOrderDAta,
        client_id: String,
    },

    #[serde(rename = "ON_RAMP")]
    OnRame { data: OnRampData, client_id: String },

    #[serde(rename = "GET_DEPTH")]
    GetDepth {
        data: GetDepthData,
        client_id: String,
    },

    #[serde(rename = "GET_OPEN_ORDERS")]
    GetOpenOrders {
        data: GetOpenOrdersData,
        client_id: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOrderData {
    pub market: String,
    pub price: String,
    pub quantity: String,
    pub side: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelOrderDAta {
    pub order_id: String,
    pub market: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OnRampData {
    pub amount: String,
    pub user_id: String,
    pub txn_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetDepthData {
    pub market: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetOpenOrdersData {
    pub user_id: String,
    pub market: String,
}

//message to api
#[derive(Deserialize, Debug, Serialize)]
#[serde(tag = "type")]
pub enum MessageToApi {
    #[serde(rename = "DEPTH")]
    Depth { payload: DepthPayload },

    #[serde(rename = "ORDER_PLACED")]
    OrderPlaced { payload: OrderPlacedPayload },

    #[serde(rename = "ORDER_CANCELLED")]
    OrderCancelled { payload: OrderCancelledPayload },

    #[serde(rename = "OPEN_ORDERS")]
    OpenOrders { payload: OpenOrdersPayload },
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DepthPayload {
    pub bids: Vec<String, String>,
    pub asks: Vec<String, String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct OrderPlacedPayload {
    pub order_id: String,
    pub executed_qty: Decimal,
    pub fills: Vec<FillInfo>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct FillInfo {
    pub price: String,
    pub qty: String,
    pub trade_id: u64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct OrderCancelledPayload {
    pub order_id: String,
    pub executed_qty: Decimal,
    pub remaining_qty: Decimal,
}

// #[derive(Deserialize, Debug, Serialize)]
// pub struct OpenOrdersPayload {}
