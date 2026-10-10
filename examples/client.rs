//! Talks to `examples/server.rs`: a few TCP requests, then a UDP batch.
//!
//! ```sh
//! cargo run --example client --features tokio
//! ```

use acrudjson::client::{
    self, Batch, Compute, ComputeInto, Create, Delete, Read, Response, Update,
};
use acrudjson::op::{Divide, Multiply};
use acrudjson::{BigDecimal, Call, V1, V2};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UdpSocket};

const SERVER: &str = "127.0.0.1:9999";

/// Sends one request over TCP and prints both sides.
async fn send<C: Call>(
    stream: &mut BufReader<TcpStream>,
    id: u64,
    call: &C,
) -> std::io::Result<()> {
    let mut request = Vec::new();
    // Odd ids use 2.0, even ids 1.0. The same server answers both.
    if id % 2 == 1 {
        client::request::<V2, C>(&mut request, id, call);
    } else {
        client::request::<V1, C>(&mut request, id, call);
    }
    request.push(b'\n');
    stream.write_all(&request).await?;
    let mut line = String::new();
    stream.read_line(&mut line).await?;
    println!("-> {}", String::from_utf8_lossy(&request).trim_end());
    println!("<- {}", line.trim_end());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let g: BigDecimal = "0.000000000066731039356729".parse()?;
    let mass: BigDecimal = "6416930923733925522307001.29472615".parse()?;
    let half: BigDecimal = "0.5".parse()?;

    let mut stream = BufReader::new(TcpStream::connect(SERVER).await?);
    // Start clean so the example can be rerun.
    send(&mut stream, 1, &Delete { key: "G" }).await?;
    send(&mut stream, 2, &Delete { key: "M" }).await?;
    send(
        &mut stream,
        3,
        &Create {
            key: "G",
            value: &g,
        },
    )
    .await?;
    send(
        &mut stream,
        4,
        &Create {
            key: "M",
            value: &mass,
        },
    )
    .await?;
    send(
        &mut stream,
        5,
        &ComputeInto {
            op: Multiply,
            lhs: "G",
            rhs: "M",
            into: "GM",
        },
    )
    .await?;
    send(
        &mut stream,
        6,
        &Compute {
            op: Multiply,
            lhs: "M",
            rhs: &half,
        },
    )
    .await?;
    send(&mut stream, 7, &Read { key: "missing" }).await?;

    // A JSON-RPC 2.0 batch in one UDP datagram. Notifications get no reply.
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    socket.connect(SERVER).await?;
    let mut request = Vec::new();
    Batch::request(&mut request, 10, &Read { key: "GM" })
        .and_request(
            11,
            &Compute {
                op: Divide,
                lhs: "GM",
                rhs: &BigDecimal::from(0),
            },
        )
        .and_notification(&Update {
            key: "G",
            value: &half,
        })
        .finish();
    socket.send(&request).await?;
    let mut datagram = vec![0; acrudjson::server::MAX_DATAGRAM];
    let len = socket.recv(&mut datagram).await?;
    println!("-> {}", String::from_utf8_lossy(&request));
    for response in Response::parse_many(&datagram[..len])? {
        match response.number() {
            Ok(Some(n)) => println!("<- id {}: {n}", response.id),
            Ok(None) => println!(
                "<- id {}: {}",
                response.id,
                response.result().map_or("?", |r| r.get())
            ),
            Err(e) => println!("<- id {}: error {e}", response.id),
        }
    }
    Ok(())
}
