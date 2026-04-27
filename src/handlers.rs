use crate::db::insert_or_update_product;
use crate::models::{Adjust, AppState, NewProduct, PriceUpdate, Product};
use actix_web::{get, web, HttpResponse, Responder};
use actix_web_actors::ws;
use rusqlite::Connection;
use serde_json;
use std::sync::Arc;

#[get("/")]
pub async fn index() -> impl Responder {
    log::info!("Serving index");
    let html = r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Welcome to Shopdrop</title>
    <style>
      :root {
        --primary-color: #4f46e5;
        --secondary-color: #06b6d4;
        --background: #f8fafc;
        --card-bg: #ffffff;
        --text: #1e293b;
        --text-muted: #64748b;
        --border: #e2e8f0;
        --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
        --shadow-hover: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
      }

      * {
        box-sizing: border-box;
      }

      body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
        background: var(--background);
        color: var(--text);
        margin: 0;
        padding: 0;
        line-height: 1.6;
      }

      header {
        background: linear-gradient(135deg, var(--primary-color), var(--secondary-color));
        color: #fff;
        padding: 2rem 1rem;
        text-align: center;
        box-shadow: var(--shadow);
      }

      header h1 {
        margin: 0;
        font-size: 2.5rem;
        font-weight: 700;
      }

      @media (max-width: 768px) {
        header h1 {
          font-size: 2rem;
        }
      }

      #products {
        list-style: none;
        padding: 2rem 1rem;
        margin: 0 auto;
        max-width: 1200px;
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 1.5rem;
      }

      #products li {
        background: var(--card-bg);
        padding: 1.5rem;
        border-radius: 12px;
        box-shadow: var(--shadow);
        transition: all 0.3s ease;
        border: 1px solid var(--border);
      }

      #products li:hover {
        transform: translateY(-4px);
        box-shadow: var(--shadow-hover);
      }

      .name {
        font-size: 1.25rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
        color: var(--text);
      }

      .price {
        font-weight: 700;
        font-size: 1.5rem;
        color: var(--primary-color);
        margin-bottom: 0.5rem;
      }

      .inventory {
        font-size: 0.9rem;
        color: var(--text-muted);
      }

      form {
        max-width: 500px;
        margin: 2rem auto;
        background: var(--card-bg);
        padding: 2rem;
        border-radius: 12px;
        box-shadow: var(--shadow);
        border: 1px solid var(--border);
      }

      form h2 {
        margin-top: 0;
        color: var(--text);
        font-size: 1.5rem;
        text-align: center;
      }

      form div {
        margin-bottom: 1rem;
      }

      label {
        display: block;
        margin-bottom: 0.5rem;
        font-weight: 500;
        color: var(--text);
      }

      input {
        width: 100%;
        padding: 0.75rem;
        border: 1px solid var(--border);
        border-radius: 8px;
        font-size: 1rem;
        transition: border-color 0.2s, box-shadow 0.2s;
      }

      input:focus {
        outline: none;
        border-color: var(--primary-color);
        box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.1);
      }

      button {
        width: 100%;
        background: linear-gradient(135deg, var(--primary-color), var(--secondary-color));
        color: #fff;
        border: none;
        padding: 0.75rem 1rem;
        border-radius: 8px;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: transform 0.2s, box-shadow 0.2s;
      }

      button:hover {
        transform: translateY(-2px);
        box-shadow: var(--shadow-hover);
      }

      button:active {
        transform: translateY(0);
      }

      @media (max-width: 768px) {
        #products {
          grid-template-columns: 1fr;
          padding: 1rem;
        }

        form {
          margin: 1rem;
          padding: 1.5rem;
        }

        header {
          padding: 1.5rem 1rem;
        }
      }
    </style>
  </head>
  <body>
    <header>
      <h1>Shopdrop — Live Inventory and product prices</h1>
    </header>
    <ul id="products"></ul>
    <form id="add-product-form">
      <h2>Add New Product</h2>
      <div>
        <label for="sku">SKU:</label>
        <input type="text" id="sku" name="sku" required>
      </div>
      <div>
        <label for="name">Name:</label>
        <input type="text" id="name" name="name" required>
      </div>
      <div>
        <label for="price">Price:</label>
        <input type="number" id="price" name="price" step="0.01" required>
      </div>
      <div>
        <label for="inventory">Inventory:</label>
        <input type="number" id="inventory" name="inventory" required>
      </div>
      <button type="submit">Add Product</button>
    </form>
    <script>
      const productList = document.getElementById('products');

      const renderProduct = (p) => {
        const id = 'p-' + p.id;
        let el = document.getElementById(id);
        if (!el) {
          el = document.createElement('li');
          el.id = id;
          productList.appendChild(el);
        }
        el.innerHTML = `
          <div class="name">${p.name}</div>
          <div class="price">$${p.price.toFixed(2)}</div>
          <div class="inventory">${p.inventory} in stock</div>
          <button class="edit-price">Edit Price</button>
        `;
        el.querySelector('.edit-price').addEventListener('click', async () => {
          const updated = parseFloat(prompt('Enter new price for ' + p.name + ':', p.price));
          if (isNaN(updated) || updated < 0) {
            alert('Please enter a valid non-negative price.');
            return;
          }
          try {
            const resp = await fetch('/api/price', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ sku: p.id, price: updated })
            });
            if (!resp.ok) {
              alert('Error updating price: ' + resp.statusText);
            }
          } catch (err) {
            alert('Error updating price: ' + err.message);
          }
        });
      };

      const ws = new WebSocket((location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + location.host + '/ws');
      ws.onmessage = (ev) => {
        const data = JSON.parse(ev.data);
        if (data.type === 'update') {
          const p = data.product;
          renderProduct(p);
        }
      };

      const fetchProducts = async () => {
        try {
          const response = await fetch('/api/products');
          if (response.ok) {
            const products = await response.json();
            products.forEach(renderProduct);
          }
        } catch (err) {
          console.warn('Could not fetch product list:', err);
        }
      };

      fetchProducts();

      document.getElementById('add-product-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const formData = new FormData(e.target);
        const product = {
          sku: formData.get('sku'),
          name: formData.get('name'),
          price: parseFloat(formData.get('price')),
          inventory: parseInt(formData.get('inventory'))
        };
        try {
          const response = await fetch('/api/products', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(product)
          });
          if (response.ok) {
            e.target.reset();
            alert('Product added successfully!');
          } else {
            alert('Error adding product: ' + response.statusText);
          }
        } catch (error) {
          alert('Error: ' + error.message);
        }
      });
    </script>
  </body>
</html>
"#;

    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}

pub async fn ws_index(
    req: actix_web::HttpRequest,
    stream: web::Payload,
    data: web::Data<Arc<AppState>>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let rx = data.broadcaster.subscribe();
    let session = crate::websocket::WsSession { rx };
    ws::start(session, &req, stream)
}

pub async fn list_products(data: web::Data<Arc<AppState>>) -> impl Responder {
    let map: tokio::sync::RwLockReadGuard<'_, std::collections::HashMap<String, Product>> = data.products.read().await;
    let vals: Vec<&Product> = map.values().collect();
    HttpResponse::Ok().json(vals)
}

pub async fn adjust_inventory(
    adj: web::Json<Adjust>,
    data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, actix_web::Error> {
    let mut map: tokio::sync::RwLockWriteGuard<'_, std::collections::HashMap<String, Product>> = data.products.write().await;
    if let Some(prod) = map.get_mut(&adj.sku) {
        if adj.delta < 0 {
            let dec = adj.delta.abs() as u32;
            prod.inventory = prod.inventory.saturating_sub(dec);
        } else {
            prod.inventory = prod.inventory.saturating_add(adj.delta as u32);
        }

        let prod_clone = prod.clone();
        let db_sku = adj.sku.clone();
        tokio::task::spawn_blocking(move || {
            let conn = match Connection::open("shopdrop.db") {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to open DB for adjust_inventory: {}", e);
                    return;
                }
            };
            if let Err(e) = insert_or_update_product(&conn, &db_sku, &prod_clone) {
                log::error!("Failed to update product in DB: {}", e);
            }
        });

        let msg = serde_json::json!({"type":"update","product":prod});
        if let Err(e) = data.broadcaster.send(msg.to_string()) {
            log::debug!("No active WebSocket listeners for adjust_inventory: {}", e);
        }

        Ok(HttpResponse::Ok().json(prod))
    } else {
        Ok(HttpResponse::NotFound().body("sku not found"))
    }
}

pub async fn update_price(
    update: web::Json<PriceUpdate>,
    data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, actix_web::Error> {
    let mut map: tokio::sync::RwLockWriteGuard<'_, std::collections::HashMap<String, Product>> = data.products.write().await;
    if let Some(prod) = map.get_mut(&update.sku) {
        prod.price = update.price;

        let prod_clone = prod.clone();
        let db_sku = update.sku.clone();
        tokio::task::spawn_blocking(move || {
            let conn = match Connection::open("shopdrop.db") {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to open DB for update_price: {}", e);
                    return;
                }
            };
            if let Err(e) = insert_or_update_product(&conn, &db_sku, &prod_clone) {
                log::error!("Failed to update product price in DB: {}", e);
            }
        });

        let msg = serde_json::json!({"type":"update","product":prod});
        if let Err(e) = data.broadcaster.send(msg.to_string()) {
            log::debug!("No active WebSocket listeners for update_price: {}", e);
        }

        Ok(HttpResponse::Ok().json(prod))
    } else {
        Ok(HttpResponse::NotFound().body("sku not found"))
    }
}

pub async fn add_product(
    new_prod: web::Json<NewProduct>,
    data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, actix_web::Error> {
    let mut map: tokio::sync::RwLockWriteGuard<'_, std::collections::HashMap<String, Product>> = data.products.write().await;
    if map.contains_key(&new_prod.sku) {
        return Ok(HttpResponse::Conflict().body("sku already exists"));
    }
    let product = Product {
        id: new_prod.sku.clone(), // using sku as id for simplicity
        name: new_prod.name.clone(),
        price: new_prod.price,
        inventory: new_prod.inventory,
    };
    map.insert(new_prod.sku.clone(), product.clone());

    // persist to sqlite
    let prod_clone = product.clone();
    let db_sku = new_prod.sku.clone();
    tokio::task::spawn_blocking(move || {
        let conn = match Connection::open("shopdrop.db") {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to open DB for add_product: {}", e);
                return;
            }
        };
        if let Err(e) = insert_or_update_product(&conn, &db_sku, &prod_clone) {
            log::error!("Failed to insert product in DB: {}", e);
        }
    });

    let msg = serde_json::json!({"type":"update","product":&product});
    if let Err(e) = data.broadcaster.send(msg.to_string()) {
        log::debug!("No active WebSocket listeners for add_product: {}", e);
    }

    Ok(HttpResponse::Created().json(product))
}