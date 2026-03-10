# ShopDrop

A real-time web application for tracking product inventory and prices with live updates.

## Features

- Real-time product price and inventory updates via WebSockets
- Web interface displaying products with live changes
- API endpoints for listing, adding, and adjusting product inventory
- SQLite database for persistent data storage
- Background simulation of price and inventory fluctuations
- Simple HTML interface with embedded JavaScript

## Installation

Ensure you have Rust installed. Then clone the repository and build the project:

```bash
git clone https://github.com/russ021/shopdrop_app.git
cd shopdrop_app
cargo build --release
```

## Usage

Run the application:

```bash
cargo run
```

Open your browser to `http://127.0.0.1:8080` to view the live product dashboard.

### API Endpoints

- `GET /` - Main web interface
- `GET /api/products` - List all products (JSON)
- `POST /api/products` - Add a new product (JSON body: `{"sku": "string", "name": "string", "price": number, "inventory": number}`)
- `POST /api/adjust` - Adjust inventory for a product (JSON body: `{"sku": "string", "delta": number}`)
- `GET /ws` - WebSocket endpoint for real-time updates

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
