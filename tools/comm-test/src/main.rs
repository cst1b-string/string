use clap::Parser;
use comm::{peer::PeerState, Socket};
use std::{net::SocketAddr, time::Duration};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

/// comm-test is a simple tool to test the string-comm crate.
#[derive(Debug, Parser)]
struct Args {
    /// The source port to bind to.
    bind_port: u16,
    /// The destination IP address to add as a peer.
    peer_addr: SocketAddr,
    /// Whether to initiate the connection.
    #[clap(long)]
    initiate: bool,
}

#[tokio::main]
async fn main() {
    let Args {
        bind_port,
        initiate,
        peer_addr,
    } = Args::parse();

    // initialise tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::TRACE.into())
                .from_env_lossy(),
        )
        .init();

    // bind to the socket
    let mut socket = match Socket::bind(([0, 0, 0, 0], bind_port).into()).await {
        Ok(s) => s,
        Err(_) => {
            error!("[-] Failed to bind to local.");
            return;
        }
    };

    // add peer
    let (_app_outbound_tx, _app_inbound_rx) = socket.add_peer(peer_addr, initiate).await;

    info!("Setup success");
    info!("Attempting transmission...");

    let mut i: u16 = 0;
    let mut ready: bool = false;

    // Wait 5 mins
    while i < 5 * 60 * 2 && !ready {
        match socket.get_peer_state(peer_addr).await {
            None => {}
            Some(s) => {
                if s == PeerState::Established {
                    ready = true;
                }
            }
        }
        i += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // check if the connection is ready
    if !ready {
        error!("[-] Connection failure.");
        return;
    }

    info!("[+] Connection with {0} succeeded!", peer_addr);
}
