use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone, Serialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub inventory: u32,
}

pub struct AppState {
    pub products: RwLock<HashMap<String, Product>>,
    pub broadcaster: broadcast::Sender<String>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            products: RwLock::new(HashMap::new()),
            broadcaster: tx,
        }
    }
}

#[derive(Deserialize)]
pub struct Adjust {
    pub sku: String,
    pub delta: i32,
}

#[derive(Deserialize)]
pub struct PriceUpdate {
    pub sku: String,
    pub price: f64,
}

#[derive(Deserialize)]
pub struct NewProduct {
    pub sku: String,
    pub name: String,
    pub price: f64,
    pub inventory: u32,
}