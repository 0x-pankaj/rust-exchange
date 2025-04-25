use actix_web::{App, HttpServer, web};

use routes::order::order_router;

mod models;
mod redis_manager;
mod routes;
mod types;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server is running on port 8000");
    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(web::scope("/api/v1").configure(order_router))
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
