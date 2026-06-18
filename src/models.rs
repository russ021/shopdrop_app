use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Draft,
    Review,
    Approved,
    Rejected,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewStatus::Draft => "draft",
            ReviewStatus::Review => "review",
            ReviewStatus::Approved => "approved",
            ReviewStatus::Rejected => "rejected",
        }
    }

    pub fn is_public(&self) -> bool {
        matches!(self, ReviewStatus::Approved)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub inventory: u32,
    pub brand: String,
    pub model: String,
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warranty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    pub status: ReviewStatus,
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
    pub brand: String,
    pub model: String,
    pub condition: String,
    pub warranty: Option<String>,
    pub review_notes: Option<String>,
    pub price: f64,
    pub inventory: u32,
}

#[derive(Deserialize)]
pub struct ReviewAction {
    pub sku: String,
    pub action: String,
    pub notes: Option<String>,
}
