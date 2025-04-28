use serde::{Deserialize, Serialize};

pub const TRADE_ADDED: &str = "TRADE_ADDED";
pub const ORDER_UPDATE: &str = "ORDER_UPDATE";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DbMessage {
    TradeAdded { data: TradeAddedData },
    OrderUpdate { data: OrderUpdateData },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeAddedData {
    pub id: String,
    pub is_buyer_maket: bool,
    pub price: String,
    pub quantity: String,
    pub quote_quantity: String,
    pub timestamp: u64,
    pub market: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderUpdateData {
    pub order_id: String,
    pub executed_qty: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}
