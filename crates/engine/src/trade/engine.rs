use actix_web::{guard::Patch, http::header::Quality};
use rand::{Rng, distributions::Alphanumeric, thread_rng};
use rust_decimal::Decimal;
use std::{collections::HashMap, result, str::FromStr};

use crate::{
    models::{
        balance::{AssetBalance, UserBalance},
        order::{Fill, Order, OrderSide},
    },
    redis_manager::{self, redis_manager::RedisManager},
    types::api::{
        CancelOrderDAta, CreateOrderData, FillInfo, GetDepthData, GetOpenOrdersData,
        MessageFromApi, MessageToApi, OnRampData, OrderPlacedPayload,
    },
};

use super::orderbook::{BASE_CURRENCY, Orderbook};

pub struct Engine {
    orderbooks: Vec<Orderbook>,
    balances: HashMap<String, UserBalance>,
}

impl Engine {
    pub fn new() -> Self {
        let mut engine = Engine {
            orderbooks: Vec::new(),
            balances: HashMap::new(),
        };
        engine
        // will implement snap shot later
    }

    pub async fn process(self, message: MessageFromApi) {
        match message {
            MessageFromApi::CreateOrder { data, client_id } => {}
        }
    }

    //handling create order function
    async fn handle_create_order(&mut self, data: CreateOrderData, client_id: &str) {
        let market = data.market;
        let price_str = data.price;
        let quantity_str = data.quantity;
        let side_str = data.side;
        let user_id = data.user_id;

        //we get sell and buy in string formate
        let side = if side_str == "buy" {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };

        match self
            .create_order(&market, &price_str, &quantity_str, side, &user_id)
            .await
        {
            Ok((executed_qty, fills, order_id)) => {
                let fill_infos = fills
                    .into_iter()
                    .map(|f| FillInfo {
                        price: f.price,
                        qty: f.qty,
                        trade_id: f.trade_id,
                    })
                    .collect();

                let redis_manager = RedisManager::get_instance().await;
                let manager = redis_manager.lock().await;

                let message = MessageToApi::OrderPlaced {
                    payload: OrderPlacedPayload {
                        order_id,
                        executed_qty,
                        fills: fill_infos,
                    },
                };

                if let Err(e) = manager.send_to_api(client_id, message).await {}
            }
        }
    }

    async fn create_order(
        &mut self,
        market: &str,
        price_str: &str,
        quantity_str: &str,
        side: OrderSide,
        user_id: &str,
    ) -> Result<(Decimal, Vec<Fill>, String), Box<dyn std::error::Error>> {
        let orderbook = self
            .orderbooks
            .iter_mut()
            .find(|o| o.ticker() == market)
            .ok_or("No orderbook found")?;

        let parts: Vec<&str> = market.split('_').collect();
        let base_asset = parts[0];
        let quote_asset = parts.get(1).unwrap_or(&BASE_CURRENCY);

        let price = Decimal::from_str(price_str)?;
        let quantity = Decimal::from_str(quantity_str)?;

        // Check and lock funds
        self.check_and_lock_funds(base_asset, quote_asset, &side, user_id, price, quantity)?;

        // Generate a random order ID
        let order_id = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect::<String>();

        let order = Order {
            price,
            quantity,
            order_id: order_id.clone(),
            filled: Decimal::ZERO,
            side,
            user_id: user_id.to_string(),
        };

        let result = orderbook.add_order(order);

        Ok((result.executed_qty, result.fills, order_id))
    }

    fn check_and_lock_funds(
        &mut self,
        base_asset: &str,
        quote_asset: &str,
        side: &OrderSide,
        user_id: &str,
        price: Decimal,
        quantity: Decimal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match side {
            OrderSide::Buy => {
                let required_funds = price * quantity;

                // Get or create user balance
                if !self.balances.contains_key(user_id) {
                    self.balances.insert(user_id.to_string(), HashMap::new());
                }

                let user_balance = self.balances.get_mut(user_id).unwrap();

                // Get or create quote asset balance
                if !user_balance.contains_key(quote_asset) {
                    user_balance.insert(
                        quote_asset.to_string(),
                        AssetBalance::new(Decimal::ZERO, Decimal::ZERO),
                    );
                }

                let asset_balance = user_balance.get_mut(quote_asset).unwrap();

                // Check if enough funds are available
                if asset_balance.available < required_funds {
                    return Err("Insufficient funds".into());
                }

                // Lock funds
                asset_balance.available -= required_funds;
                asset_balance.locked += required_funds;
            }
            OrderSide::Sell => {
                // Get or create user balance
                if !self.balances.contains_key(user_id) {
                    self.balances.insert(user_id.to_string(), HashMap::new());
                }

                let user_balance = self.balances.get_mut(user_id).unwrap();

                // Get or create base asset balance
                if !user_balance.contains_key(base_asset) {
                    user_balance.insert(
                        base_asset.to_string(),
                        AssetBalance::new(Decimal::ZERO, Decimal::ZERO),
                    );
                }

                let asset_balance = user_balance.get_mut(base_asset).unwrap();

                // Check if enough assets are available
                if asset_balance.available < quantity {
                    return Err("Insufficient assets".into());
                }

                // Lock assets
                asset_balance.available -= quantity;
                asset_balance.locked += quantity;
            }
        }

        Ok(())
    }

    fn update_balance() {}

    fn create_db_trades() {}

    fn update_db_orders() {}

    fn publish_ws_depth_updates() {}

    fn publish_ws_trades() {}

    async fn handle_cancel_order(&mut self, data: CancelOrderDAta, client_id: &str) {}

    async fn handle_get_open_orders(&mut self, data: GetOpenOrdersData, client_id: &str) {}

    async fn handle_on_ramp(&mut self, data: OnRampData, client_id: &str) {}

    async fn handle_get_depth(&mut self, data: GetDepthData, client_id: &str) {}
}
