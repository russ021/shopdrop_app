use crate::db::insert_or_update_product;
use crate::models::{AppState, Product, ReviewStatus};
use rand::prelude::Rng;
use rusqlite::Connection;
use serde_json;
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn start_simulation(state: Arc<AppState>, tx: broadcast::Sender<String>) {
    let mut rng = rand::rng();
    let mut next_sku_id = 300;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        // Hold the write lock for one simulation tick so inventory changes and
        // newly generated products are observed consistently by API requests.
        let mut map: tokio::sync::RwLockWriteGuard<'_, std::collections::HashMap<String, Product>> = state.products.write().await;

        if rng.random_bool(0.15) {
            let templates = [
                (
                    "Outlet Power Bank",
                    "ChargeCo",
                    "PB-100",
                    "Open-box",
                    "30-day",
                    29.99,
                    6,
                    "Incoming inventory for inspection.",
                ),
                (
                    "Outlet Bluetooth Speaker",
                    "SoundWave",
                    "SPK-250",
                    "Refurbished",
                    "90-day",
                    49.99,
                    4,
                    "Quality check pending.",
                ),
                (
                    "Outlet Smart Charger",
                    "Voltix",
                    "SC-45",
                    "Like New",
                    "60-day",
                    24.99,
                    8,
                    "Needs final review.",
                ),
            ];

            let chosen = &templates[rng.random_range(0..templates.len())];
            let (name, brand, model, condition, warranty, price, inventory, notes) = chosen;

            // Avoid flooding the review queue with duplicate pending templates.
            if map.values().any(|p| p.status == ReviewStatus::Review && p.name == *name) {
                continue;
            }

            let sku = format!("sku-{}", next_sku_id);
            next_sku_id += 1;
            let new_product = Product {
                id: sku.clone(),
                name: name.to_string(),
                price: *price,
                inventory: *inventory,
                brand: brand.to_string(),
                model: model.to_string(),
                condition: condition.to_string(),
                warranty: Some(warranty.to_string()),
                review_notes: Some(notes.to_string()),
                status: ReviewStatus::Review,
            };
            map.insert(sku.clone(), new_product.clone());
            let msg = serde_json::json!({"type":"update","product":new_product});
            let _ = tx.send(msg.to_string());
        }

        for (sku, prod) in map.iter_mut() {
            if !prod.status.is_public() {
                continue;
            }
            if rng.random_bool(0.3) && prod.inventory > 0 {
                let dec = rng.random_range(0..=2);
                prod.inventory = prod.inventory.saturating_sub(dec);
            }
            let prod_clone = prod.clone();
            let sku_clone = sku.clone();
            // rusqlite is synchronous, so database writes run off the async
            // executor while the in-memory update is broadcast immediately.
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
