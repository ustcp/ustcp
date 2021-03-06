#[macro_use]
extern crate log;
use argh::FromArgs;
use futures::StreamExt;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing_subscriber;
use tracing_subscriber::EnvFilter;
use ustunet;
use ustunet::TcpListener;

#[derive(FromArgs)]
/// Reach one server with arbitrary socket addresses.
struct ConnectUp {
    /// address of server to connect to
    #[argh(positional)]
    server: SocketAddr,

    /// tun device owned by current user
    #[argh(option)]
    tun: String,
}

#[tokio::main]
async fn main() {
    let _subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let up: ConnectUp = argh::from_env();
    let server = up.server;
    let mut listener = TcpListener::bind(&up.tun).unwrap();
    println!("Listening on {}", up.tun);
    while let Some(socket) = listener.next().await {
        tokio::spawn(async move {
            match copy_to_server(server, socket).await {
                Ok((s, r)) => info!("Received {} bytes, sent {}", r, s),
                Err(error) => error!("Error while copying: {:?}", error),
            }
        });
    }
}

async fn copy_to_server(
    remote: SocketAddr,
    socket: ustunet::stream::TcpStream,
) -> io::Result<(u64, u64)> {
    info!(
        "Accepted new tcp stream from {:?} to {:?}",
        socket.peer_addr(),
        socket.local_addr()
    );
    let server = TcpStream::connect(&remote).await?;
    info!("Connected to {:?}", remote);
    let (mut reader, mut writer) = server.into_split();
    let (mut client_reader, mut client_writer) = socket.split();
    let sent = tokio::spawn(async move {
        let n = tokio::io::copy(&mut reader, &mut client_writer).await;
        info!("Sent bytes: {:?}", n);
        n
    });
    let recv = tokio::spawn(async move {
        let n = tokio::io::copy(&mut client_reader, &mut writer).await;
        info!("Received bytes: {:?}", n);
        n
    });
    let (sent, recv) = tokio::join!(sent, recv);
    let sent = sent??;
    let recv = recv??;
    Ok((sent, recv))
}
