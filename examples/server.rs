//! Serves JSON-RPC over UDP and TCP on 127.0.0.1:9999.
//!
//! ```sh
//! cargo run --example server --features tokio
//! ```

use acrudjson::Store;
use acrudjson::server::{serve_tcp, serve_udp};

use std::sync::Arc;

use tokio::net::{TcpListener, UdpSocket};

const ADDR: &str = "127.0.0.1:9999";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join("acrudjson-example.redb");
    let store = Arc::new(Store::open(&path)?);
    let namespace: Arc<str> = Arc::from("default");
    println!("store: {}", path.display());
    println!("listening on udp://{ADDR} and tcp://{ADDR} (one JSON message per line)");

    let udp = serve_udp(
        store.clone(),
        UdpSocket::bind(ADDR).await?,
        namespace.clone(),
    );
    let tcp = serve_tcp(store, TcpListener::bind(ADDR).await?, namespace);
    tokio::try_join!(udp, tcp)?;
    Ok(())
}
