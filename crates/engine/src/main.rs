use actix_web;
use log::info;
use std::env;

mod trade;
mod types;
mod models;
mod redis_manager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    println!("Starting trading engine...");
    dotenv::dotenv().ok();

    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".to_string());


    //redis message loop
    let mut redis_client =

}
