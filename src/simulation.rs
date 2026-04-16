use crate::db::insert_or_update_product;
use crate::models::{AppState, Product};
use rand::Rng;
use rusqlite::Connection;
use serde_json;
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn start_simulation(state: Arc<AppState>, tx: broadcast::Sender<String>) {
    let mut rng = rand::rng();
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let mut map: tokio::sync::RwLockWriteGuard<'_, std::collections::HashMap<String, Product>> = state.products.write().await;
        for (sku, prod) in map.iter_mut() {
            // random price change +/- up to 5%
            let change = (rng.random_range(-50i32..51) as f64) / 1000.0; // -5%..+5%
            prod.price = (prod.price * (1.0 + change) * 100.0).round() / 100.0;
            // random inventory change -0..2
            if rng.random_bool(0.3) && prod.inventory > 0 {
                let dec = rng.random_range(0..=2);
                prod.inventory = prod.inventory.saturating_sub(dec);
            }

            // persist update to sqlite (blocking)
            let prod_clone = prod.clone();
            let sku_clone = sku.clone();
            tokio::task::spawn_blocking(move || {
                let conn = match Connection::open("shopdrop.db") {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to open DB connection: {}", e);
                        return;
                    }
                };
                if let Err(e) = insert_or_update_product(&conn, &sku_clone, &prod_clone) {
                    log::error!("Failed to update product in DB: {}", e);
                }
            });

            let msg = serde_json::json!({"type":"update","product":prod});
            if let Err(e) = tx.send(msg.to_string()) {
                log::debug!("No active WebSocket listeners for simulation update: {}", e);
            }
        }
    }
}