use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WsMessage {
    TickerUpdate(TicketUpdateMessage),
    DepthUpdate(DepthUpdateMessage),
    TradeAdded(TradeUpdateMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TicketUpdateMessage {
    pub stream: String,
    pub data: TickerUpdateData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TickerUpdateData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub V: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<String>,
    pub id: u64,
    pub e: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepthUpdateMessage {
    stream: String,
    data: DepthUpdateData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepthUpdateData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<Vec<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<Vec<String, String>>,
    pub e: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUpdateMessage {
    stream: string,
    data: TradeUpdateData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUpdateData {
    pub e: String, //name trade
    pub t: u64,
    pub m: bool,
    pub p: String,
    pub q: String,
    pub s: String, //symbol
}
