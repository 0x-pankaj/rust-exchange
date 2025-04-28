use log::error;
use rand::{Rng, distributions::Alphanumeric, thread_rng};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::{collections::HashMap, str::FromStr};

use crate::{
    models::{
        balance::{AssetBalance, UserBalance},
        order::{Fill, Order, OrderSide},
    },
    redis_manager::redis_manager::RedisManager,
    types::{
        api::{
            CancelOrderDAta, CreateOrderData, DepthPayload, FillInfo, GetDepthData,
            GetOpenOrdersData, MessageFromApi, MessageToApi, OnRampData, OrderCancelledPayload,
            OrderPlacedPayload,
        },
        db::{DbMessage, OrderUpdateData, TradeAddedData},
        ws::{DepthUpdateData, DepthUpdateMessage, TradeAddedMessage, WsMessage, WsTradeAddedData},
    },
};

use super::orderbook::{BASE_CURRENCY, Orderbook};

pub struct Engine {
    orderbooks: Vec<Orderbook>,
    balances: HashMap<String, UserBalance>,
}

impl Engine {
    pub fn new() -> Self {
        let engine = Engine {
            orderbooks: Vec::new(),
            balances: HashMap::new(),
        };
        engine
        // will implement snap shot later
    }

    pub async fn process(&mut self, message: MessageFromApi) {
        match message {
            MessageFromApi::CreateOrder { data, client_id } => {
                self.handle_create_order(data, &client_id).await;
            }
            MessageFromApi::CancelOrder { data, client_id } => {
                self.handle_cancel_order(data, &client_id).await;
            }
            MessageFromApi::GetOpenOrders { data, client_id } => {
                self.handle_get_open_orders(data, &client_id).await;
            }
            MessageFromApi::OnRame { data, client_id: _ } => {
                self.handle_on_ramp(data).await;
            }
            MessageFromApi::GetDepth { data, client_id } => {
                self.handle_get_depth(data, &client_id).await
            }
        }
    }

    async fn handle_get_depth(&self, data: GetDepthData, client_id: &str) {
        let market = data.market;

        if let Some(orderbook) = self.orderbooks.iter().find(|o| o.ticker() == market) {
            let (bids, asks) = orderbook.get_depth();

            let redis_manager = RedisManager::get_instance();

            let message = MessageToApi::Depth {
                payload: DepthPayload { bids, asks },
            };

            if let Err(e) = redis_manager.send_to_api(client_id, message).await {
                error!("Failed to send depth message: {}", e);
            }
        } else {
            error!("Orderbook not found for market: {}", market);

            let redis_manager = RedisManager::get_instance();
            let message = MessageToApi::Depth {
                payload: DepthPayload {
                    bids: Vec::new(),
                    asks: Vec::new(),
                },
            };

            if let Err(e) = redis_manager.send_to_api(client_id, message).await {
                error!("Failed to send empth depth message: {}", e);
            }
        }
    }

    //handling create order function
    async fn handle_create_order(&mut self, data: CreateOrderData, client_id: &str) {
        let market = data.market;
        let price_str = data.price;
        let quantity_str = data.quantity;
        let side_str = data.side;
        let user_id = data.user_id;

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
                        qty: f.qty.to_string(),
                        trade_id: f.trade_id,
                    })
                    .collect();

                let manager = RedisManager::get_instance();

                let message = MessageToApi::OrderPlaced {
                    payload: OrderPlacedPayload {
                        order_id,
                        executed_qty,
                        fills: fill_infos,
                    },
                };

                if let Err(e) = manager.send_to_api(client_id, message).await {
                    error!("Failed to send order placed message: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to create order: {}", e);

                let manager = RedisManager::get_instance();

                let message = MessageToApi::OrderCancelled {
                    payload: OrderCancelledPayload {
                        order_id: "".to_string(),
                        executed_qty: Decimal::ZERO,
                        remaining_qty: Decimal::ZERO,
                    },
                };

                if let Err(e) = manager.send_to_api(client_id, message).await {
                    error!("Failed to send order cancelled message: {}", e);
                }
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
        let parts: Vec<&str> = market.split("_").collect();
        let base_asset = parts[0];
        let quote_asset = parts.get(1).unwrap_or(&BASE_CURRENCY);
        let price = Decimal::from_str(price_str)?;
        let quantity = Decimal::from_str(quantity_str)?;
        self.check_and_lock_funds(base_asset, quote_asset, &side, user_id, price, quantity)?;

        let order_id = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect::<String>();

        let orderbook = self
            .orderbooks
            .iter_mut()
            .find(|o| o.ticker() == market)
            .ok_or("No orderbook found")?;

        // Generate a random order ID

        let order = Order {
            price,
            quantity,
            order_id: order_id.clone(),
            filled: Decimal::ZERO,
            side: side.clone(),
            user_id: user_id.to_string(),
        };

        let order_clone = order.clone();

        let result = orderbook.add_order(order);

        //updating balance based on fills
        self.update_balance(user_id, base_asset, quote_asset, &side, &result.fills);

        //creating database record for trades
        self.create_db_trades(&result.fills, market).await;

        //updating database
        self.update_db_orders(&order_clone, result.executed_qty, &result.fills, market)
            .await;

        //publish websocket depth updates

        self.publish_ws_depth_updates(&result.fills, price_str, &side, market)
            .await;

        //publish websocket trades
        self.publish_ws_trades(&result.fills, user_id, market).await;

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

    fn update_balance(
        &mut self,
        user_id: &str,
        base_asset: &str,
        quote_asset: &str,
        side: &OrderSide,
        fills: &[Fill],
        // executed_qty: Decimal,
    ) {
        match side {
            OrderSide::Buy => {
                for fill in fills {
                    let fill_price = Decimal::from_str(&fill.price).unwrap_or(Decimal::ZERO);
                    let fill_value = fill.qty * fill_price;

                    // asset , base , quote

                    // seller gets quote currency
                    if let Some(maker_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(asset_balance) = maker_balance.get_mut(quote_asset) {
                            asset_balance.available += fill_value;
                        }
                    }

                    //buyers locked funds are decreased
                    if let Some(taker_balance) = self.balances.get_mut(user_id) {
                        if let Some(asset_balance) = taker_balance.get_mut(quote_asset) {
                            asset_balance.locked -= fill_value;
                        }
                    }

                    //seller locked base decreases
                    if let Some(maker_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(asset_balance) = maker_balance.get_mut(base_asset) {
                            asset_balance.locked -= fill.qty;
                        }
                    }

                    //buyers get base currency
                    if let Some(taker_balance) = self.balances.get_mut(user_id) {
                        if let Some(asset_balance) = taker_balance.get_mut(base_asset) {
                            asset_balance.available += fill.qty;
                        } else {
                            //create base asset balance if it doesn't exist
                            taker_balance.insert(
                                base_asset.to_string(),
                                AssetBalance {
                                    available: fill.qty,
                                    locked: Decimal::ZERO,
                                },
                            );
                        }
                    }
                }
            }
            OrderSide::Sell => {
                for fill in fills {
                    let fill_price = Decimal::from_str(&fill.price).unwrap_or(Decimal::ZERO);
                    let fill_value = fill_price * fill.qty;

                    //buyer locked quote get decrease
                    if let Some(maker_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(asset_balance) = maker_balance.get_mut(quote_asset) {
                            asset_balance.locked -= fill_value;
                        }
                    }

                    //seller get quote currency
                    if let Some(taker_balance) = self.balances.get_mut(user_id) {
                        if let Some(asset_balance) = taker_balance.get_mut(quote_asset) {
                            asset_balance.available += fill_value;
                        } else {
                            //creating quote asset balance if not exits
                            taker_balance.insert(
                                quote_asset.to_string(),
                                AssetBalance {
                                    available: fill_value,
                                    locked: Decimal::ZERO,
                                },
                            );
                        }
                    }

                    //buyer get base currency
                    if let Some(maker_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(asset_balance) = maker_balance.get_mut(base_asset) {
                            asset_balance.available += fill.qty;
                        }
                    }

                    //sellers locked base decrease
                    if let Some(taker_balance) = self.balances.get_mut(user_id) {
                        if let Some(asset_balance) = taker_balance.get_mut(base_asset) {
                            asset_balance.locked -= fill.qty;
                        }
                    }
                }
            }
        }
    }

    async fn create_db_trades(&self, fills: &[Fill], market: &str) {
        let redis_manager = RedisManager::get_instance();

        for fill in fills {
            let qty_decimal = fill.qty;
            let price_decimal = Decimal::from_str(&fill.price).unwrap_or(Decimal::ZERO);
            let quote_qty = qty_decimal * price_decimal;

            let message = DbMessage::TradeAdded {
                data: TradeAddedData {
                    id: fill.trade_id.to_string(),
                    is_buyer_maket: true, //todo here to check if this is correct
                    price: fill.price.clone(),
                    quantity: fill.qty.to_string(),
                    quote_quantity: quote_qty.to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    market: market.to_string(),
                },
            };

            if let Err(e) = redis_manager.push_message(message).await {
                error!("Failed to push trade added message: {}", e)
            }
        }
    }

    async fn update_db_orders(
        &mut self,
        ordr: &Order,
        executed_qty: Decimal,
        fills: &[Fill],
        market: &str,
    ) {
        let redis_manager = RedisManager::get_instance();

        //updating the taker message
        let message = DbMessage::OrderUpdate {
            data: OrderUpdateData {
                order_id: ordr.order_id.clone(),
                executed_qty: executed_qty.to_f64().unwrap_or(0.0),
                market: Some(market.to_string()),
                price: Some(ordr.price.to_string()),
                quantity: Some(ordr.quantity.to_string()),
                side: Some(match ordr.side {
                    OrderSide::Buy => "buy".to_string(),
                    OrderSide::Sell => "sell".to_string(),
                }),
            },
        };

        if let Err(e) = redis_manager.push_message(message).await {
            error!("Failed to push order update message: {}", e);
        }

        //update maker order
        for fill in fills {
            let message = DbMessage::OrderUpdate {
                data: OrderUpdateData {
                    order_id: fill.marker_order_id.clone(),
                    executed_qty: fill.qty.to_f64().unwrap_or(0.0),
                    market: None,
                    price: None,
                    quantity: None,
                    side: None,
                },
            };

            if let Err(e) = redis_manager.push_message(message).await {
                error!("Failed to push update message for fill: {}", e);
            }
        }
    }

    async fn publish_ws_depth_updates(
        &self,
        fills: &[Fill],
        price: &str,
        side: &OrderSide,
        market: &str,
    ) {
        if let Some(orderbook) = self.orderbooks.iter().find(|o| o.ticker() == market) {
            let (bids, asks) = orderbook.get_depth();

            match side {
                OrderSide::Buy => {
                    let fill_prices: Vec<String> = fills.iter().map(|f| f.price.clone()).collect();
                    let updated_asks: Vec<(String, String)> = asks
                        .into_iter()
                        .filter(|(p, _)| fill_prices.contains(p))
                        .collect();

                    let updated_bids = bids.into_iter().find(|(p, _)| p == price);

                    let redis_manager = RedisManager::get_instance();

                    let message = WsMessage::DepthUpdate(DepthUpdateMessage {
                        stream: format!("depth@{}", market),
                        data: DepthUpdateData {
                            a: Some(updated_asks),
                            b: updated_bids.map(|b| vec![b]).or(Some(vec![])),
                            e: "depth".to_string(),
                        },
                    });
                    if let Err(e) = redis_manager
                        .publish_message(&format!("depth@{}", market), message)
                        .await
                    {
                        error!("Failed to publish depth update message: {}", e);
                    }
                }
                OrderSide::Sell => {
                    let fill_prices: Vec<String> = fills.iter().map(|f| f.price.clone()).collect();
                    let updated_bids: Vec<(String, String)> = bids
                        .into_iter()
                        .filter(|(p, _)| fill_prices.contains(p))
                        .collect();

                    let updated_asks = asks.into_iter().find(|(p, _)| p == price);
                    let redis_manager = RedisManager::get_instance();
                    let message = WsMessage::DepthUpdate(DepthUpdateMessage {
                        stream: format!("depth@{}", market),
                        data: DepthUpdateData {
                            a: updated_asks.map(|a| vec![a]).or(Some(vec![])),
                            b: Some(updated_bids),
                            e: "depth".to_string(),
                        },
                    });

                    if let Err(e) = redis_manager
                        .publish_message(&format!("depth@{}", market), message)
                        .await
                    {
                        error!("Failed to publish depth message: {}", e);
                    }
                }
            }
        }
    }

    async fn send_updated_depth_at(&self, price: &str, market: &str) {
        if let Some(orderbook) = self.orderbooks.iter().find(|o| o.ticker() == market) {
            let (bids, asks) = orderbook.get_depth();

            let updated_bids: Vec<(String, String)> =
                bids.into_iter().filter(|(p, _)| p == price).collect();

            let updated_asks: Vec<(String, String)> =
                asks.into_iter().filter(|(p, _)| p == price).collect();

            let redis_manager = RedisManager::get_instance();

            let message = WsMessage::DepthUpdate(DepthUpdateMessage {
                stream: format!("depth@{}", market),
                data: DepthUpdateData {
                    a: if updated_asks.is_empty() {
                        Some(vec![(price.to_string(), "0".to_string())])
                    } else {
                        Some(updated_asks)
                    },
                    b: if (updated_bids.is_empty()) {
                        Some(vec![(price.to_string(), "0".to_string())])
                    } else {
                        Some(updated_bids)
                    },
                    e: "depth".to_string(),
                },
            });

            if let Err(e) = redis_manager
                .publish_message(&format!("depth@{}", market), message)
                .await
            {
                error!("Failed to publish depth update message: {}", e);
            }
        }
    }

    async fn publish_ws_trades(&self, fills: &[Fill], user_id: &str, market: &str) {
        let redis_manager = RedisManager::get_instance();

        for fill in fills {
            let message = WsMessage::TradeAdded(TradeAddedMessage {
                stream: format!("trade@{}", market),
                data: WsTradeAddedData {
                    e: "trade".to_string(),
                    t: fill.trade_id,
                    m: fill.other_user_id == user_id,
                    p: fill.price.clone(),
                    q: fill.qty.to_string(),
                    s: market.to_string(),
                },
            });

            if let Err(e) = redis_manager
                .publish_message(&format!("trade@{}", market), message)
                .await
            {
                error!("Failed to publish trade message: {}", e);
            }
        }
    }

    async fn handle_cancel_order(&mut self, data: CancelOrderDAta, client_id: &str) {
        let order_id = data.order_id;
        let market = data.market;

        if let Some(orderbook) = self.orderbooks.iter_mut().find(|o| o.ticker() == market) {
            let quote_asset = market.split('_').nth(1).unwrap_or(BASE_CURRENCY);

            // Find the order in either bids or asks
            let order = orderbook
                .bids
                .iter()
                .find(|o| o.order_id == order_id)
                .or_else(|| orderbook.asks.iter().find(|o| o.order_id == order_id))
                .cloned();

            if let Some(order) = order {
                let price_opt = if order.side == OrderSide::Buy {
                    // Cancel bid
                    let price = orderbook.cancel_bid(&order);
                    // Unlock funds
                    if let Some(balance) = self.balances.get_mut(&order.user_id) {
                        if let Some(asset_balance) = balance.get_mut(BASE_CURRENCY) {
                            let left_quantity = (order.quantity - order.filled) * order.price;
                            asset_balance.available += left_quantity;
                            asset_balance.locked -= left_quantity;
                        }
                    }
                    price
                } else {
                    // Cancel ask
                    let price = orderbook.cancel_ask(&order);
                    // Unlock funds
                    if let Some(balance) = self.balances.get_mut(&order.user_id) {
                        if let Some(asset_balance) = balance.get_mut(quote_asset) {
                            let left_quantity = order.quantity - order.filled;
                            asset_balance.available += left_quantity;
                            asset_balance.locked -= left_quantity;
                        }
                    }
                    price
                };

                // Update depth if price level changed
                if let Some(price) = price_opt {
                    self.send_updated_depth_at(&price.to_string(), &market)
                        .await;
                }

                // Send confirmation to client
                let manager = RedisManager::get_instance();
                let message = MessageToApi::OrderCancelled {
                    payload: OrderCancelledPayload {
                        order_id,
                        executed_qty: Decimal::ZERO,
                        remaining_qty: Decimal::ZERO,
                    },
                };

                if let Err(e) = manager.send_to_api(client_id, message).await {
                    error!("Failed to send order cancelled message: {}", e);
                }
            } else {
                error!("Order not found: {}", order_id);
            }
        } else {
            error!("Orderbook not found for market: {}", market);
        }
    }

    async fn handle_get_open_orders(&mut self, data: GetOpenOrdersData, client_id: &str) {
        let market = data.market;
        let user_id = data.user_id;

        if let Some(orderbook) = self.orderbooks.iter().find(|o| o.ticker() == market) {
            let open_orders = orderbook.get_open_orders(&user_id);

            let redis_manager = RedisManager::get_instance();

            let message = MessageToApi::OpenOrders {
                payload: open_orders,
            };

            if let Err(e) = redis_manager.send_to_api(client_id, message).await {
                error!("Failed to send open orders message: {}", e);
            } else {
                error!("Orderbook not found for market: {}", market);
            }
        }
    }

    async fn handle_on_ramp(&mut self, data: OnRampData) {
        let user_id = data.user_id;

        let amount = Decimal::from_str(&data.amount).unwrap_or_else(|_| Decimal::ZERO);

        self.on_ramp(&user_id, amount);
    }

    fn on_ramp(&mut self, user_id: &str, amount: Decimal) {
        if !self.balances.contains_key(user_id) {
            let mut balance = HashMap::new();

            balance.insert(
                BASE_CURRENCY.to_string(),
                AssetBalance::new(amount, Decimal::ZERO),
            );
        } else if let Some(user_balance) = self.balances.get_mut(user_id) {
            if !user_balance.contains_key(BASE_CURRENCY) {
                user_balance.insert(
                    BASE_CURRENCY.to_string(),
                    AssetBalance::new(amount, Decimal::ZERO),
                );
            } else if let Some(asset_balance) = user_balance.get_mut(BASE_CURRENCY) {
                asset_balance.available += amount;
            }
        }
    }

    fn set_base_balances(&mut self) {
        let users = ["1", "2", "3"];
        let assets = [BASE_CURRENCY, "TATA"];
        let initial_amount = Decimal::from(10_000_000);

        for user_id in users.iter() {
            let mut user_balance = HashMap::new();

            for asset in assets.iter() {
                user_balance.insert(
                    asset.to_string(),
                    AssetBalance::new(initial_amount, Decimal::ZERO),
                );
            }

            self.balances.insert(user_id.to_string(), user_balance);
        }
    }
}
