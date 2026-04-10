use actix_web::{web, App, HttpServer};
use env_logger;
use log;
use shopdrop::db;
use shopdrop::handlers;
use shopdrop::models::{AppState, Product};
use shopdrop::simulation;
use std::collections::HashMap;
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut products_map = HashMap::new();
    products_map.insert(
        "sku-101".to_string(),
        Product {
            id: "101".to_string(),
            name: "Rusty Coffee Mug".to_string(),
            price: 12.50,
            inventory: 42,
        },
    );
    products_map.insert(
        "sku-202".to_string(),
        Product {
            id: "202".to_string(),
            name: "Linux T-Shirt".to_string(),
            price: 24.99,
            inventory: 13,
        },
    );

    // Initialize DB
    if let Err(e) = db::init_db(&products_map) {
        log::error!("Failed to initialize DB: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DB init failed"));
    }

    let state = AppState::new();
    {
        let mut map = state.products.write().await;
        *map = products_map;
    }
    let state = Arc::new(state);

    // Start simulation
    let state_clone = state.clone();
    let tx_clone = state.broadcaster.clone();
    actix_rt::spawn(async move {
        simulation::start_simulation(state_clone, tx_clone).await;
    });

    log::info!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(handlers::index)
            .route("/api/products", web::get().to(handlers::list_products))
            .route("/api/products", web::post().to(handlers::add_product))
            .route("/api/adjust", web::post().to(handlers::adjust_inventory))
            .route("/ws", web::get().to(handlers::ws_index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
