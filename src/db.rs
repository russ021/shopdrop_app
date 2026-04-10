use crate::models::Product;
use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::HashMap;

pub fn init_db(products: &HashMap<String, Product>) -> SqlResult<()> {
    let conn = Connection::open("shopdrop.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS products (
            sku TEXT PRIMARY KEY,
            id TEXT,
            name TEXT,
            price REAL,
            inventory INTEGER
        )",
        [],
    )?;

    for (sku, prod) in products.iter() {
        insert_or_update_product(&conn, sku, prod)?;
    }
    Ok(())
}

pub fn insert_or_update_product(conn: &Connection, sku: &str, product: &Product) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO products (sku, id, name, price, inventory) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(sku) DO UPDATE SET price = excluded.price, inventory = excluded.inventory",
        params![sku, product.id, product.name, product.price, product.inventory],
    )?;
    Ok(())
}