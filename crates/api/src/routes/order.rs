use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

use crate::{
    redis_manager::redis_manager::RedisManager,
    types::messages::{CANCEL_ORDER, CREATE_ORDER, GET_OPEN_ORDERS, MessageToEngine, OrderSide},
};

#[derive(Deserialize)]
pub struct CreateOrderRequest {
    market: String,
    price: String,
    quantity: String,
    side: OrderSide,
    user_id: String,
}

#[derive(Deserialize)]
pub struct CancelOrderRequest {
    order_id: String,
    market: String,
}

#[derive(Deserialize)]
pub struct GetOpenOrdersQuery {
    user_id: String,
    market: String,
}

pub fn order_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/order")
            .route(web::post().to(create_order))
            .route(web::delete().to(cancel_order)),
    );

    cfg.service(web::resource("/order/open").route(web::get().to(get_open_orders)));
}

async fn create_order(data: web::Json<CreateOrderRequest>) -> impl Responder {
    let redis = RedisManager::get_instance();
    let message = MessageToEngine {
        type_: CREATE_ORDER.to_string(),
        data: serde_json::json!({
            "market": data.market,
            "price": data.price,
            "quantity": data.quantity,
            "side": data.side,
            "userId": data.user_id
        }),
    };

    match redis.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn get_open_orders(query: web::Query<GetOpenOrdersQuery>) -> impl Responder {
    let redis = RedisManager::get_instance();
    let message = MessageToEngine {
        type_: GET_OPEN_ORDERS.to_string(),
        data: serde_json::json!({
            "userId": query.user_id,
            "market": query.market
        }),
    };

    match redis.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn cancel_order(data: web::Json<CancelOrderRequest>) -> impl Responder {
    let redis = RedisManager::get_instance();
    let message = MessageToEngine {
        type_: CANCEL_ORDER.to_string(),
        data: serde_json::json!({
            "orderId": data.order_id,
            "market": data.market
        }),
    };

    match redis.send_and_await(message).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
