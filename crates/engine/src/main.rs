use redis::AsyncCommands;
use serde_json;
use std::time::Duration;
use tokio::time;
use trade::engine;

mod models;
mod redis_manager;
mod trade;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    log::info!("Strting trading engine");

    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut redis_conn = redis_client.get_async_connection().await?;

    log::info!("Connected to Redis");

    loop {
        let response: Option<String> = redis_conn.rpop("messages").await?;
        if let Some(message) = response {
            match serde_json::from_str(&message) {
                Ok(parsed) => {
                    engine.process(parsed);
                }
                Err(e) => {
                    log::error!("failed to parse message: {}", e);
                }
            }
        }

        time::sleep(Duration::from_millis(10)).await;
    }
}
