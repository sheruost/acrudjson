use acrudjson::prelude::v1::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use tokio::{
    net::UdpSocket,
    time::{sleep, timeout},
};
use zerocopy::{AsBytes, ByteSlice, LittleEndian, Ref, U32};

type Checksum = U32<LittleEndian>;

#[repr(C)]
struct DatagramPayload<B> {
    body: B,
    checksum: Ref<B, Checksum>,
}

impl<B: ByteSlice> DatagramPayload<B> {
    fn parse(bytes: B) -> Option<DatagramPayload<B>> {
        let (body, checksum) = Ref::new_unaligned_from_suffix(bytes)?;
        Some(DatagramPayload { body, checksum })
    }

    fn get_checksum(&self) -> u32 {
        self.checksum.get()
    }

    fn get_request_body(&self) -> Result<RespBody, ClientError> {
        let respbody: RespBody = serde_json::from_slice(&self.body)?;
        Ok(respbody)
    }
}

const SERVER_PORT: u16 = 9999;
const CLIENT_PORT: u16 = 9998;
const UDP_DATAGRAM_MAX_SIZE: usize = 65536;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let sock = Arc::new(UdpSocket::bind(format!("0.0.0.0:{CLIENT_PORT}")).await?);
    let recv_sock = sock.clone();
    // running recv socket at background.
    let recv_task = tokio::spawn(async move {
        let mut databuf = vec![0_u8; UDP_DATAGRAM_MAX_SIZE];
        while let Ok((len, peer_addr)) = recv_sock.recv_from(&mut databuf).await {
            let payload = databuf[..len].to_vec();
            if let Some(resp_payload) = DatagramPayload::parse(payload.as_bytes()) {
                let checksum = crc32fast::hash(&resp_payload.body);
                if checksum == resp_payload.get_checksum() {
                    // use unwrap since we verified checksum.
                    let body = resp_payload.get_request_body().unwrap();
                    info!(
                        "Server JSON Response: \n{}",
                        serde_json::to_string(&body).unwrap()
                    );
                } else {
                    error!("checksum unmatched.");
                }
            } else {
                error!("unrecognisable datagram payload from {peer_addr}");
            }
        }
    });
    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
    sock.connect(server_addr).await?;
    let data = r#"
        {
            "jsonrpc": "1.0",
            "method": "create",
            "params": [
                "grav_const",
                "0.000000000066731039356729"
            ],
            "id": 1
        }
        "#;
    let req_payload = RequestBuilder::from_json(data)?.build()?;
    sock.send(&req_payload).await?;
    info!("Client JSON Request: {data}");
    sleep(Duration::from_secs(1)).await;
    let data2 = r#"
        {
            "jsonrpc": "1.0",
            "method": "create",
            "params": [
                "planet_mass",
                "6416930923733925522307001.29472615"
            ],
            "id": 2
        }
        "#;
    let req_payload2 = RequestBuilder::from_json(data2)?.build()?;
    sock.send(&req_payload2).await?;
    info!("Client JSON Request: {data2}");
    sleep(Duration::from_secs(1)).await;
    let data3 = r#"
        {
            "jsonrpc":"1.0",
            "method":"multiply",
            "params":["grav_const", "planet_mass"],
            "id":3
        }
    "#;
    let req_payload3 = RequestBuilder::from_json(data3)?.build()?;
    sock.send(&req_payload3).await?;
    info!("Client JSON Request: {data3}");
    sleep(Duration::from_secs(1)).await;
    let data4 = r#"
        {
            "jsonrpc":"1.0",
            "method":"multiply",
            "params":["planet_mass", "0.5"],
            "id":4
        }
    "#;
    let req_payload4 = RequestBuilder::from_json(data4)?.build()?;
    sock.send(&req_payload4).await?;
    info!("Client JSON Request: {data4}");
    sleep(Duration::from_secs(1)).await;
    let data5 = r#"
        {
            "jsonrpc":"1.0",
            "method":"update",
            "params":["grav_const", "428208470021099.94"],
            "id":5
        }
    "#;
    let req_payload5 = RequestBuilder::from_json(data5)?.build()?;
    sock.send(&req_payload5).await?;
    info!("Client JSON Request: {data5}");
    sleep(Duration::from_secs(1)).await;
    let data6 = r#"
        {
            "jsonrpc":"1.0",
            "method":"delete",
            "params":["grav_const"],
            "id":6
        }
    "#;
    let req_payload6 = RequestBuilder::from_json(data6)?.build()?;
    sock.send(&req_payload6).await?;
    info!("Client JSON Request: {data6}");
    timeout(Duration::from_secs(10), recv_task).await??;
    Ok(())
}
