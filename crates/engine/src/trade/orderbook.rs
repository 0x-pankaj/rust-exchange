use std::collections::HashMap;

use crate::models::order::{Fill, Order, OrderSide};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const BASE_CURRENCY: &str = "INR";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderMatchResult {
    pub executed_qty: Decimal,
    pub fills: Vec<Fill>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Orderbook {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub base_asset: String,
    pub quote_asset: String,
    pub last_trade_id: u64,
    pub current_price: Decimal,
}

impl Orderbook {
    pub fn new(
        base_asset: String,
        bids: Vec<Order>,
        asks: Vec<Order>,
        last_trade_id: u64,
        current_price: Decimal,
    ) -> Self {
        Self {
            bids,
            asks,
            base_asset,
            quote_asset: BASE_CURRENCY.to_string(),
            last_trade_id,
            current_price,
        }
    }

    pub fn ticker(&self) -> String {
        format!("{}_{}", self.base_asset, self.quote_asset)
    }

    pub fn get_snapshot(&self) -> Self {
        self.clone()
    }

    pub fn add_order(&mut self, mut order: Order) -> OrderMatchResult {
        match order.side {
            OrderSide::Buy => {
                let result = self.match_bid(&order);
                order.filled = result.executed_qty;

                if order.filled < order.quantity {
                    self.bids.push(order);
                }

                result
            }

            OrderSide::Sell => {
                let result = self.match_ask(&order);
                order.filled = result.executed_qty;

                if order.filled < order.quantity {
                    self.asks.push(order);
                }
                result
            }
        }
    }

    fn match_bid(&mut self, order: &Order) -> OrderMatchResult {
        let mut fills = Vec::new();
        let mut executed_qty: Decimal = Decimal::ZERO;

        //sorting ask price lowest first
        self.asks.sort_by(|a, b| a.price.cmp(&b.price));

        let mut i = 0;
        while i < self.asks.len() {
            if self.asks[i].price <= order.price && executed_qty < order.quantity {
                let filled_qty = std::cmp::min(
                    (order.quantity - executed_qty),
                    (self.asks[i].quantity - self.asks[i].filled),
                );

                executed_qty += filled_qty;
                self.asks[i].filled += filled_qty;

                fills.push(Fill {
                    price: self.asks[i].price.to_string(),
                    qty: filled_qty,
                    trade_id: self.last_trade_id,
                    other_user_id: self.asks[i].user_id.clone(),
                    marker_order_id: self.asks[i].order_id.clone(),
                });

                self.last_trade_id += 1;
            }

            i += 1;
        }

        //  removing fills order
        self.asks.retain(|order| order.filled < order.quantity);

        OrderMatchResult {
            executed_qty,
            fills,
        }
    }

    fn match_ask(&mut self, order: &Order) -> OrderMatchResult {
        let mut fills = Vec::new();
        let mut executed_qty: Decimal = Decimal::ZERO;

        //sorting ask by price

        self.asks.sort_by(|a, b| a.price.cmp(&b.price));

        let mut i = 0;

        while i < self.asks.len() {
            if self.asks[i].price <= order.price && executed_qty < order.quantity {
                let filled_qty = std::cmp::min(
                    (order.quantity - executed_qty),
                    (self.asks[i].quantity - self.asks[i].filled),
                );

                executed_qty += filled_qty;
                self.asks[i].filled += filled_qty;

                fills.push(Fill {
                    price: self.asks[i].price.to_string(),
                    qty: filled_qty,
                    trade_id: self.last_trade_id,
                    other_user_id: self.asks[i].user_id.clone(),
                    marker_order_id: self.asks[i].order_id.clone(),
                });

                self.last_trade_id += 1;
            }
            i += 1;
        }

        //removing filled orders

        self.asks.retain(|order| order.filled < order.quantity);

        OrderMatchResult {
            executed_qty,
            fills,
        }
    }

    pub fn get_depth(&self) -> (Vec<(String, String)>, Vec<(String, String)>) {
        let mut bids: HashMap<String, Decimal> = HashMap::new();
        let mut asks: HashMap<String, Decimal> = HashMap::new();

        //aggegrating order of same price
        for order in &self.bids {
            let price = order.price.to_string();
            let remaining = order.quantity - order.filled;
            *bids.entry(price).or_insert(Decimal::ZERO) += remaining;
        }

        for order in &self.asks {
            let price = order.price.to_string();
            let remaining = order.quantity - order.filled;
            *asks.entry(price).or_insert(Decimal::ZERO) += remaining;
        }

        //converting vectors to tuples

        let bids_vec: Vec<(String, String)> = bids
            .iter()
            .map(|(price, qty)| (price.clone(), qty.to_string()))
            .collect();

        let ask_vec: Vec<(String, String)> = asks
            .iter()
            .map(|(price, qty)| (price.clone(), qty.to_string()))
            .collect();

        (bids_vec, ask_vec)
    }

    pub fn get_open_orders(&self, user_id: &str) -> Vec<Order> {
        let mut orders = Vec::new();

        for order in &self.asks {
            if order.user_id == user_id && order.filled < order.quantity {
                orders.push(order.clone());
            }
        }

        for order in &self.bids {
            if order.user_id == user_id && order.filled < order.quantity {
                orders.push(order.clone());
            }
        }

        orders
    }

    //canceling a bid order

    pub fn cancel_bid(&mut self, order: &Order) -> Option<Decimal> {
        if let Some(pos) = self.bids.iter().position(|o| o.order_id == order.order_id) {
            let price = self.bids[pos].price;
            self.bids.remove(pos);
            Some(price)
        } else {
            None
        }
    }

    //cancel an ask order
    pub fn cancel_ask(&mut self, order: &Order) -> Option<Decimal> {
        if let Some(pos) = self.asks.iter().position(|o| o.order_id == order.order_id) {
            let price = self.asks[pos].price;
            self.asks.remove(pos);
            Some(price)
        } else {
            None
        }
    }
}
