use actix::prelude::*;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone, Serialize)]
struct Product {
    id: String,
    name: String,
    price: f64,
    inventory: u32,
}

struct AppState {
    products: RwLock<HashMap<String, Product>>,
    broadcaster: broadcast::Sender<String>,
}

struct WsSession {
    rx: broadcast::Receiver<String>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        let mut rx = self.rx.resubscribe();

        // Spawn a tokio task that forwards broadcast messages to this actor
        actix_rt::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        addr.do_send(ServerMessage(msg));
                    }
                    Err(_) => break,
                }
            }
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")] 
struct ServerMessage(String);

impl Handler<ServerMessage> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => (),
            Ok(ws::Message::Text(_)) => (),
            Ok(ws::Message::Binary(_)) => (),
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            _ => (),
        }
    }
}

#[get("/")]
async fn index() -> impl Responder {    println!("Serving index");    let html = r#"
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
      <h1>Shopdrop — Live Prices &amp; Inventory</h1>
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
      const ws = new WebSocket((location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + location.host + '/ws');
      ws.onmessage = (ev) => {
        const data = JSON.parse(ev.data);
        if (data.type === 'update') {
          const p = data.product;
          const id = 'p-' + p.id;
          let el = document.getElementById(id);
          if (!el) {
            el = document.createElement('li');
            el.id = id;
            document.getElementById('products').appendChild(el);
          }
          el.innerHTML = `<div class="name">${p.name}</div>` +
                         `<div class="price">$${p.price.toFixed(2)}</div>` +
                         `<div class="inventory">${p.inventory} in stock</div>`;
        }
      };

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

async fn ws_index(req: HttpRequest, stream: web::Payload, data: web::Data<Arc<AppState>>) -> actix_web::Result<actix_web::HttpResponse> {
    let rx = data.broadcaster.subscribe();
    let session = WsSession { rx };
    ws::start(session, &req, stream)
}

#[derive(Deserialize)]
struct Adjust {
    sku: String,
    delta: i32,
}

#[derive(Deserialize)]
struct NewProduct {
    sku: String,
    name: String,
    price: f64,
    inventory: u32,
}

async fn list_products(data: web::Data<Arc<AppState>>) -> impl Responder {
    let map = data.products.read().await;
    let vals: Vec<&Product> = map.values().collect();
    HttpResponse::Ok().json(vals)
}

async fn adjust_inventory(adj: web::Json<Adjust>, data: web::Data<Arc<AppState>>) -> impl Responder {
    let mut map = data.products.write().await;
    if let Some(prod) = map.get_mut(&adj.sku) {
        if adj.delta < 0 {
            let dec = adj.delta.abs() as u32;
            prod.inventory = prod.inventory.saturating_sub(dec);
        } else {
            prod.inventory = prod.inventory.saturating_add(adj.delta as u32);
        }

        let prod_clone = prod.clone();
        let db_sku = adj.sku.clone();
        // persist to sqlite in blocking task
        tokio::task::spawn_blocking(move || {
            if let Ok(conn) = Connection::open("shopdrop.db") {
                let _ = conn.execute(
                    "INSERT INTO products (sku, id, name, price, inventory) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(sku) DO UPDATE SET price = excluded.price, inventory = excluded.inventory",
                    params![db_sku, prod_clone.id, prod_clone.name, prod_clone.price, prod_clone.inventory],
                );
            }
        });

        let msg = serde_json::json!({"type":"update","product":prod});
        let _ = data.broadcaster.send(msg.to_string());

        HttpResponse::Ok().json(prod)
    } else {
        HttpResponse::NotFound().body("sku not found")
    }
}

async fn add_product(new_prod: web::Json<NewProduct>, data: web::Data<Arc<AppState>>) -> impl Responder {
    let mut map = data.products.write().await;
    if map.contains_key(&new_prod.sku) {
        return HttpResponse::Conflict().body("sku already exists");
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
        if let Ok(conn) = Connection::open("shopdrop.db") {
            let _ = conn.execute(
                "INSERT INTO products (sku, id, name, price, inventory) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![db_sku, prod_clone.id, prod_clone.name, prod_clone.price, prod_clone.inventory],
            );
        }
    });

    let msg = serde_json::json!({"type":"update","product":&product});
    let _ = data.broadcaster.send(msg.to_string());

    HttpResponse::Created().json(product)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // env_logger::init();

    let (tx, _rx) = broadcast::channel(100);

    let products_map = {
        let mut m = HashMap::new();
        m.insert(
            "sku-101".to_string(),
            Product {
                id: "101".to_string(),
                name: "Rusty Coffee Mug".to_string(),
                price: 12.50,
                inventory: 42,
            },
        );
        m.insert(
            "sku-202".to_string(),
            Product {
                id: "202".to_string(),
                name: "Linux T-Shirt".to_string(),
                price: 24.99,
                inventory: 13,
            },
        );
        m
    };

    let state = Arc::new(AppState {
        products: RwLock::new(products_map.clone()),
        broadcaster: tx.clone(),
    });

    // Initialize SQLite DB and write initial products
    {
        let _ = std::fs::create_dir_all("./");
        if let Ok(conn) = Connection::open("shopdrop.db") {
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS products (
                    sku TEXT PRIMARY KEY,
                    id TEXT,
                    name TEXT,
                    price REAL,
                    inventory INTEGER
                )",
                [],
            );

            for (sku, prod) in products_map.iter() {
                let _ = conn.execute(
                    "INSERT INTO products (sku, id, name, price, inventory) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(sku) DO UPDATE SET price = excluded.price, inventory = excluded.inventory",
                    params![sku, prod.id, prod.name, prod.price, prod.inventory],
                );
            }
        }
    }

    // Spawn a background task to simulate price/inventory changes
    {
        let state = state.clone();
        let tx = tx.clone();
        actix_rt::spawn(async move {
            let mut rng = rand::thread_rng();
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                let mut map = state.products.write().await;
                for (_k, prod) in map.iter_mut() {
                    // random price change +/- up to 5%
                    let change = (rng.gen_range(-50i32..51) as f64) / 1000.0; // -5%..+5%
                    prod.price = (prod.price * (1.0 + change) * 100.0).round() / 100.0;
                    // random inventory change -0..2
                    if rng.gen_bool(0.3) && prod.inventory > 0 {
                        let dec = rng.gen_range(0..=2);
                        prod.inventory = prod.inventory.saturating_sub(dec);
                    }

                    // persist update to sqlite (blocking)
                    let prod_clone = prod.clone();
                    let sku_clone = _k.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Ok(conn) = Connection::open("shopdrop.db") {
                            let _ = conn.execute(
                                "INSERT INTO products (sku, id, name, price, inventory) VALUES (?1, ?2, ?3, ?4, ?5)
                                 ON CONFLICT(sku) DO UPDATE SET price = excluded.price, inventory = excluded.inventory",
                                params![sku_clone, prod_clone.id, prod_clone.name, prod_clone.price, prod_clone.inventory],
                            );
                        }
                    });

                    let msg = serde_json::json!({"type":"update","product":prod});
                    let _ = tx.send(msg.to_string());
                }
            }
        });
    }

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(index)
            .route("/api/products", web::get().to(list_products))
            .route("/api/products", web::post().to(add_product))
            .route("/api/adjust", web::post().to(adjust_inventory))
            .route("/ws", web::get().to(ws_index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
