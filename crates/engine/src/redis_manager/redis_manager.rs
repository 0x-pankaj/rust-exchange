use std::sync::Arc;

use redis::{AsyncCommands, Client};
use tokio::sync::Mutex;

use crate::types::{api::MessageToApi, ws::WsMessage};



pub  struct RedisManager {
    client: Client
}


impl RedisManager {
    static mut INSTANCE: Option<Arc<Mutex<RedisManager>>> = None;

    pub fn new() -> Result<Self, redis::RedisError> {
        let client = Client::open("redis://127.0.0.1/")?;
        Ok(Self{client})
    }

    pub async fn get_instance() -> Arc<Mutex<RedisManager>> {
        unsafe {
            if INSTANCE.is_none() {
                let manager =  Self::new().expect("failed to create a redis manager");
                INSTANCE = Some(Arc::new(Mutex::new(manager)));
            }
            INSTANCE.clone().unwrap()
        }
    }

    //publishing message to database queue
    // pub async fn push_message(&self, message: )


    //publishing message for websocket queue
    pub async fn publish_message(&self, channel: &str, message: WsMessage) -> redis::RedisResult<()> {
        let mut conn = self.client.get_async_connection().await?;
        let serialized = serde_json::to_string(&message).expect("Failed to serialized WS message");

        conn.lpush("channel", serialized).await
    }

    //publishing message to api server waiting for response
    pub async  fn send_to_api(&self, client_id: &str, message: MessageToApi) -> redis::RedisResult<()> {
        let mut conn = self.client.get_async_connection().await?;
        let serialized = serde_json::to_string(&message).expect("failed to serialized api message");

        conn.publish(client_id, serialized).await
    }

}
