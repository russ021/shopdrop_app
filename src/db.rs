use crate::models::{Product, ReviewStatus};
use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::{HashMap, HashSet};

fn table_columns(conn: &Connection) -> SqlResult<HashSet<String>> {
    // Inspect the existing schema so older databases can be upgraded in place.
    let mut stmt = conn.prepare("PRAGMA table_info(products)")?;
    let rows = stmt.query_map([], |row| row.get(1))?;
    let mut columns = HashSet::new();
    for col in rows {
        columns.insert(col?);
    }
    Ok(columns)
}

pub fn init_db() -> SqlResult<()> {
    let conn = Connection::open("shopdrop.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            sku TEXT PRIMARY KEY,
            id TEXT,
            name TEXT,
            price REAL,
            inventory INTEGER,
            brand TEXT,
            model TEXT,
            condition TEXT,
            warranty TEXT,
            review_notes TEXT,
            status TEXT
        )",
        [],
    )?;

    let existing_columns = table_columns(&conn)?;
    let required_columns = [
        ("brand", "TEXT"),
        ("model", "TEXT"),
        ("condition", "TEXT"),
        ("warranty", "TEXT"),
        ("review_notes", "TEXT"),
        ("status", "TEXT"),
    ];

    // Keep startup compatible with databases created before newer product
    // metadata fields were introduced.
    for (column, ty) in required_columns {
        if !existing_columns.contains(column) {
            let sql = format!("ALTER TABLE products ADD COLUMN {} {}", column, ty);
            conn.execute(sql.as_str(), [])?;
        }
    }

    Ok(())
}

pub fn load_products() -> SqlResult<HashMap<String, Product>> {
    let conn = Connection::open("shopdrop.db")?;
    let mut stmt = conn.prepare(
        "SELECT sku, id, name, price, inventory, brand, model, condition, warranty, review_notes, status FROM products",
    )?;
    let product_iter = stmt.query_map([], |row| {
        // Nullable columns preserve compatibility with rows written by older
        // versions of the app.
        let brand: Option<String> = row.get(5)?;
        let model: Option<String> = row.get(6)?;
        let condition: Option<String> = row.get(7)?;
        let warranty: Option<String> = row.get(8)?;
        let review_notes: Option<String> = row.get(9)?;
        let status_text: Option<String> = row.get(10)?;

        Ok(Product {
            id: row.get(1)?,
            name: row.get(2)?,
            price: row.get(3)?,
            inventory: row.get(4)?,
            brand: brand.unwrap_or_else(|| "Unknown".to_string()),
            model: model.unwrap_or_else(|| "Unknown".to_string()),
            condition: condition.unwrap_or_else(|| "Unknown".to_string()),
            warranty,
            review_notes,
            status: match status_text.as_deref() {
                Some("draft") => ReviewStatus::Draft,
                Some("review") => ReviewStatus::Review,
                Some("approved") => ReviewStatus::Approved,
                Some("rejected") => ReviewStatus::Rejected,
                _ => ReviewStatus::Approved,
            },
        })
    })?;

    let mut map = HashMap::new();
    for product in product_iter {
        let prod = product?;
        // The map is keyed by the product id, which is also the public SKU in
        // the current data model.
        map.insert(prod.id.clone(), prod);
    }
    Ok(map)
}

pub fn insert_or_update_product(conn: &Connection, sku: &str, product: &Product) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO products (sku, id, name, price, inventory, brand, model, condition, warranty, review_notes, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(sku) DO UPDATE SET
            id = excluded.id,
            name = excluded.name,
            price = excluded.price,
            inventory = excluded.inventory,
            brand = excluded.brand,
            model = excluded.model,
            condition = excluded.condition,
            warranty = excluded.warranty,
            review_notes = excluded.review_notes,
            status = excluded.status",
        params![
            sku,
            product.id,
            product.name,
            product.price,
            product.inventory,
            product.brand,
            product.model,
            product.condition,
            product.warranty,
            product.review_notes,
            product.status.as_str(),
        ],
    )?;
    Ok(())
}
