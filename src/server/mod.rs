//! UDP and TCP servers (feature `tokio`).
//!
//! UDP: one message per datagram. TCP: one message per line.
//! Buffers are reused between messages.

use crate::Backend;

use std::io;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::runtime::{Handle, RuntimeFlavor};
use tokio::task::JoinSet;

mod framer;

pub use framer::LineFramer;

/// Largest UDP payload over IPv4.
pub const MAX_DATAGRAM: usize = 65_507;

/// Longest TCP line accepted, newline excluded. Longer lines close the connection.
pub const MAX_LINE: usize = 1 << 20;

/// Runs a backend call that may block on disk.
///
/// `block_in_place` instead of `spawn_blocking`, so the request can stay
/// borrowed from the read buffer.
fn blocking<R>(f: impl FnOnce() -> R) -> R {
    match Handle::current().runtime_flavor() {
        RuntimeFlavor::CurrentThread => f(),
        _ => tokio::task::block_in_place(f),
    }
}

/// Serves JSON-RPC over UDP, one receive loop per CPU.
///
/// Replies too large for a datagram are dropped.
pub async fn serve_udp<B: Backend + Send + Sync + 'static>(
    backend: Arc<B>,
    socket: UdpSocket,
    namespace: Arc<str>,
) -> io::Result<()> {
    let socket = Arc::new(socket);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut tasks = JoinSet::new();
    for _ in 0..workers {
        tasks.spawn(udp_worker(
            backend.clone(),
            socket.clone(),
            namespace.clone(),
        ));
    }
    while let Some(joined) = tasks.join_next().await {
        joined.map_err(io::Error::other)??;
    }
    Ok(())
}

async fn udp_worker<B: Backend>(
    backend: Arc<B>,
    socket: Arc<UdpSocket>,
    namespace: Arc<str>,
) -> io::Result<()> {
    let mut input = vec![0; MAX_DATAGRAM];
    let mut output = Vec::new();
    loop {
        let (len, peer) = match socket.recv_from(&mut input).await {
            Ok(received) => received,
            // Some platforms report a closed peer here. Keep serving.
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
            Err(e) => return Err(e),
        };
        output.clear();
        if blocking(|| backend.handle(&namespace, &input[..len], &mut output)) {
            let _ = socket.send_to(&output, peer).await;
        }
    }
}

/// Serves JSON-RPC over TCP, one message per line, one task per connection.
pub async fn serve_tcp<B: Backend + Send + Sync + 'static>(
    backend: Arc<B>,
    listener: TcpListener,
    namespace: Arc<str>,
) -> io::Result<()> {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionReset
                ) =>
            {
                continue;
            }
            Err(e) => return Err(e),
        };
        let (backend, namespace) = (backend.clone(), namespace.clone());
        tokio::spawn(async move {
            let _ = tcp_connection(&*backend, stream, &namespace).await;
        });
    }
}

async fn tcp_connection(
    backend: &impl Backend,
    mut stream: TcpStream,
    namespace: &str,
) -> io::Result<()> {
    let (mut reader, mut writer) = stream.split();
    let mut lines = LineFramer::new(MAX_LINE);
    let mut output = Vec::new();
    while let Some(line) = lines.next(&mut reader).await? {
        if line.trim_ascii().is_empty() {
            continue;
        }
        output.clear();
        if blocking(|| backend.handle(namespace, line, &mut output)) {
            output.push(b'\n');
            writer.write_all(&output).await?;
        }
    }
    Ok(())
}
