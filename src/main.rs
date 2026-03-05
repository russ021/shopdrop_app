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
async fn index() -> impl Responder {
    let html = r#"
<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>Shopdrop - Live</title>
  </head>
  <body>
    <h1>Shopdrop — Live Prices & Inventory</h1>
    <ul id="products"></ul>
    <script>
      const ws = new WebSocket((location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + location.host + '/ws');
      ws.onmessage = (ev) => {
        const data = JSON.parse(ev.data);
        // data: { type: 'update', product }
        if (data.type === 'update') {
          const p = data.product;
          const id = 'p-' + p.id;
          let el = document.getElementById(id);
          if (!el) {
            el = document.createElement('li');
            el.id = id;
            document.getElementById('products').appendChild(el);
          }
          el.textContent = `${p.name} — $${p.price.toFixed(2)} — ${p.inventory} in stock`;
        }
      };
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

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
            .route("/api/adjust", web::post().to(adjust_inventory))
            .route("/ws", web::get().to(ws_index))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
