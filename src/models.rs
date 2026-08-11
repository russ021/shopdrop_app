use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};

/// Review status for products in the workflow.
/// 
/// - `Draft`: Product created but not submitted for review
/// - `Review`: Product awaiting approval/rejection
/// - `Approved`: Product approved and available for purchase
/// - `Rejected`: Product rejected during review
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

/// Product in the Shopdrop inventory.
/// 
/// Represents an outlet/refurbished product with additional metadata
/// like condition, warranty, and review status.
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

/// Adjust inventory request for an existing product.
/// 
/// Used by `POST /api/adjust` to modify stock levels.
/// Delta: positive to add stock, negative to reduce stock
#[derive(Deserialize)]
pub struct Adjust {
    pub sku: String,
    pub delta: i32,
}

#[derive(Deserialize)]
/// Update product price request.
/// 
/// Used by `POST /api/price` to change the selling price.
pub struct PriceUpdate {
    pub sku: String,
    pub price: f64,
}

/// Create new product request.
/// 
/// Used by `POST /api/products` to add a new outlet product.
/// Product starts in "review" status and must be approved.
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

/// Product review action request.
/// 
/// Used by `POST /api/review` to approve or reject a product.
/// 
/// Fields:
/// - `sku`: Product SKU to review
/// - `action`: "approve" or "reject"
/// - `notes`: Optional notes (useful for rejection reasons)
#[derive(Deserialize)]
pub struct ReviewAction {
    pub sku: String,
    pub action: String,
    pub notes: Option<String>,
}
