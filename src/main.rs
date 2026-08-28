use actix_web::{web, App, HttpServer};
use env_logger;
use log;
use rusqlite::Connection;
use shopdrop::db;
use shopdrop::handlers;
use shopdrop::models::{AppState, Product, ReviewStatus};
use shopdrop::simulation;
use std::env;
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    if let Err(e) = db::init_db() {
        log::error!("Failed to initialize DB: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DB init failed"));
    }

    // Load persisted inventory before starting the server so the in-memory state
    // is immediately available to both HTTP handlers and the simulator.
    let mut products_map = db::load_products().unwrap_or_default();
    if products_map.is_empty() {
        // A fresh database gets a couple of approved products so the dashboard
        // has useful content on its first launch.
        products_map.insert(
            "sku-101".to_string(),
            Product {
                id: "sku-101".to_string(),
                name: "Outlet Wireless Headphones".to_string(),
                price: 79.99,   
                inventory: 18,
                brand: "Volt".to_string(),
                model: "VX-300".to_string(),
                condition: "Refurbished".to_string(),
                warranty: Some("90-day".to_string()),
                review_notes: Some("Inspected, cleaned, ready to ship.".to_string()),
                status: ReviewStatus::Approved,
            },
        );
        products_map.insert(
            "sku-202".to_string(),
            Product {
                id: "sku-202".to_string(),
                name: "Outlet Gaming Mouse".to_string(),
                price: 34.99,
                inventory: 9,
                brand: "Orbit".to_string(),
                model: "Turbo-X".to_string(),
                condition: "Open-box".to_string(),
                warranty: Some("60-day".to_string()),
                review_notes: Some("Tested and working. Minor box wear.".to_string()),
                status: ReviewStatus::Approved,
            },
        );

        let conn = Connection::open("shopdrop.db").map_err(|e| {
            log::error!("Failed to open DB for seed persistence: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "DB seed failed")
        })?;

        for (sku, prod) in products_map.iter() {
            if let Err(e) = db::insert_or_update_product(&conn, sku, prod) {
                log::error!("Failed to seed DB product {}: {}", sku, e);
            }
        }
    }

    let state = AppState::new();
    {
        // Populate the lock before sharing state with request handlers.
        let mut map = state.products.write().await;
        *map = products_map;
    }
    let state = Arc::new(state);

    // Run inventory changes independently of the HTTP server. Updates are sent
    // through the broadcaster so every connected dashboard receives them.
    let state_clone = state.clone();
    let tx_clone = state.broadcaster.clone();
    actix_rt::spawn(async move {
        simulation::start_simulation(state_clone, tx_clone).await;
    });

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(7878);

    log::info!("Starting server at http://127.0.0.1:{}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(handlers::index)
            .route("/api/products", web::get().to(handlers::list_products))
            .route("/api/products", web::post().to(handlers::add_product))
            .route("/api/products/pending", web::get().to(handlers::list_pending_products))
            .route("/api/review", web::post().to(handlers::review_product))
            .route("/api/price", web::post().to(handlers::update_price))
            .route("/api/adjust", web::post().to(handlers::adjust_inventory))
            .route("/ws", web::get().to(handlers::ws_index))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
