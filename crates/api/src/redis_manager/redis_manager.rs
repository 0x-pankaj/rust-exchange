use std::sync::Arc;

use crate::types::messages::{MessageFromOrderbook, MessageToEngine};
use futures::StreamExt;

use rand::{Rng, distributions::Alphanumeric};
use redis::{AsyncCommands, Client};

pub struct RedisManager {
    client: Client,
    publisher: Client,
}

// Defining  the static instance with once_cell
// static INSTANCE: Lazy<Mutex<RedisManager>> = Lazy::new(|| Mutex::new(RedisManager::new()));

impl RedisManager {
    fn new() -> Self {
        let client = Client::open("redis://127.0.0.1/").expect("Failed to create Redis client");
        let publisher =
            Client::open("redis://127.0.0.1/").expect("Failed to create Redis publisher");
        RedisManager { client, publisher }
    }

    pub fn get_instance() -> Arc<RedisManager> {
        static INSTANCE: once_cell::sync::Lazy<Arc<RedisManager>> =
            once_cell::sync::Lazy::new(|| Arc::new(RedisManager::new()));
        INSTANCE.clone()
    }

    pub async fn send_and_await(
        &self,
        message: MessageToEngine,
    ) -> anyhow::Result<MessageFromOrderbook> {
        let client_id = self.get_random_client_id();
        let conn = self.client.get_async_connection().await?;
        let mut pub_conn = self.publisher.get_async_connection().await?;

        // Subscribing to the client_id channel
        let mut pubsub = conn.into_pubsub();
        pubsub.subscribe(&client_id).await?;
        let mut stream = pubsub.on_message();

        // Pushing message to Redis queue with client ID
        let message_data = serde_json::json!({
            "clientId": client_id,
            "message": message
        })
        .to_string();

        // Explicitly specify the return type as i64 (Redis returns the new length of the list)
        let _: i64 = pub_conn.lpush("messages", message_data).await?;

        // Wait for response on the subscribed channel
        let msg = stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No message received"))?;

        // Explicitly converting to String to satisfy FromRedisValue trait
        let payload: String = redis::from_redis_value(&msg.get_payload()?)?;

        // Parsing the response
        let response: MessageFromOrderbook = serde_json::from_str(&payload)?;

        Ok(response)
    }

    fn get_random_client_id(&self) -> String {
        let mut rng = rand::thread_rng();
        let first_part: String = (0..13).map(|_| rng.sample(Alphanumeric) as char).collect();
        let second_part: String = (0..13).map(|_| rng.sample(Alphanumeric) as char).collect();
        first_part + &second_part
    }
}
