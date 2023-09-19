use acrudjson::prelude::v1::*;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use tokio::{net::UdpSocket, runtime::Builder, time::timeout};
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

    fn get_request_body(&self) -> Result<ReqBody, ServerError> {
        let reqbody: ReqBody = serde_json::from_slice(&self.body)?;
        Ok(reqbody)
    }
}

const SERVER_PORT: u16 = 9999;
const UDP_DATAGRAM_MAX_SIZE: usize = 65536;

fn main() {
    env_logger::init();
    let valid_cpu_cores_count = std::thread::available_parallelism().unwrap().get();
    let rt = Builder::new_multi_thread()
        .worker_threads(valid_cpu_cores_count)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let socket = Arc::new(
            UdpSocket::bind(format!("0.0.0.0:{SERVER_PORT}"))
                .await
                .unwrap(),
        );
        info!("example UDP server running on 0.0.0.0:{SERVER_PORT}");
        let recv_sock = socket.clone();
        let pool = Arc::new(ConnectionPool::init("/tmp/jsonrpc_storage").unwrap());
        let mut datagram_buf = vec![0_u8; UDP_DATAGRAM_MAX_SIZE];
        while let Ok((len, peer_addr)) = recv_sock.recv_from(&mut datagram_buf).await {
            info!("receiving UDP datagram from {peer_addr}");
            let payload = datagram_buf[..len].to_vec();
            let pool_clone = pool.clone();
            let ttl = Duration::from_secs(5);
            let peer = peer_addr.clone();
            let send_sock = recv_sock.clone();
            tokio::spawn(async move {
                if let Err(_) = timeout(
                    ttl,
                    process(send_sock.clone(), pool_clone, peer, payload.clone()),
                )
                .await
                {
                    if let Some(parsed) = DatagramPayload::parse(payload.as_bytes()) {
                        match parsed.get_request_body() {
                            Ok(body) => {
                                let resp = ResponseBuilder::error(
                                    ErrorMsg::new(format!("server timeout.")),
                                    body.id,
                                )
                                .build();
                                match send_sock.send_to(resp.as_bytes(), peer).await {
                                    Ok(_) => info!("timeout response has been successfully sent to peer {peer}"),
                                    Err(e) => error!("failed to send timeout response, reason: {e}")
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });
        }
    });
}

//TODO: validate UserToken
async fn process(
    send_sock: Arc<UdpSocket>,
    pool: Arc<ConnectionPool>,
    peer: SocketAddr,
    payload: Vec<u8>,
) {
    if let Some(parsed) = DatagramPayload::parse(payload.as_bytes()) {
        let new_checksum = crc32fast::hash(&parsed.body);
        if new_checksum == parsed.get_checksum() {
            let req_body = parsed.get_request_body().unwrap();
            let default_user_database = pool.open_user_database("default".as_bytes()).unwrap();
            let resp_payload = default_user_database.transaction(
                req_body.parse_method(),
                req_body.parse_params(),
                req_body.id,
            );

            match send_sock.send_to(resp_payload.as_bytes(), peer).await {
                Ok(_) => info!(
                    "response ID: {} has been successfully sent to peer {}",
                    req_body.id, peer
                ),
                Err(e) => error!("failed to send response ID: {}, reason: {}", req_body.id, e),
            }
        }
    }
}
