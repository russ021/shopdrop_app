use shopdrop::db;
fn main() {
    match db::load_products() {
        Ok(products) => {
            println!("Loaded {} products", products.len());
        }
        Err(e) => {
            println!("Error loading products: {}", e);
        }
    }
}
