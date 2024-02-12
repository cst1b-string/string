use clap::Parser;
use comm::{peer::PeerState, Socket};
use std::{net::SocketAddr, time::Duration};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use protocol::{ProtocolPacket, messages, packet};
use std::io;
use tokio::sync::mpsc;

/// comm-test is a simple tool to test the string-comm crate.
#[derive(Debug, Parser)]
struct Args {
    /// The source port to bind to.
    bind_port: u16,
    /// The destination IP address to add as a peer.
    #[clap(value_delimiter = ',')]
    peer_addrs: Vec<SocketAddr>,
    /// Whether to initiate the connection.
    #[clap(long)]
    initiate: bool,
}

#[tokio::main]
async fn main() {
    let Args {
        bind_port,
        initiate,
        peer_addrs,
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

    // add peers
    let mut senders: Vec<mpsc::Sender<ProtocolPacket>> = Vec::new();
    let mut receivers: Vec<mpsc::Receiver<ProtocolPacket>> = Vec::new();

    for peer_addr in &peer_addrs {
        let (app_outbound_tx, mut app_inbound_rx) = socket.add_peer(*peer_addr, initiate).await;
        senders.push(app_outbound_tx);
        receivers.push(app_inbound_rx);
    }

    info!("[+] Setup success");
    info!("[*] Attempting transmission...");

    // Wait 5 mins
    let mut ready_peers: Vec<usize> = Vec::new();

    let mut tick: u16 = 0;

    while tick < 5 * 60 * 2 && ready_peers.len() != peer_addrs.len() {
        for i in 0..peer_addrs.len() {
            if !ready_peers.contains(&i) {
                match socket.get_peer_state(peer_addrs[i]).await {
                    None => {}
                    Some(s) => {
                        if s == PeerState::Established {
                            info!("[+] Connection with {0} succeeded!", peer_addrs[i]);
                            ready_peers.push(i);
                        }
                    }
                }
            }
        }
        tick += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // check if the connection is ready
    if ready_peers.len() != peer_addrs.len() {
        error!("[-] Connection failure.");
        return;
    }

    info!("[+] All connections succeeded!");
    info!("[+] Chat log follows below, enter any input to send:");


    tokio::task::spawn(async move {
        loop {
            for (i, mut app_inbound_rx) in receivers.iter_mut().enumerate() {
                match app_inbound_rx.try_recv() {
                    Ok(recv) => {
                        match recv.packet {
                            Some(packet::v1::packet::Packet::PktMessage(m)) => {
                                info!("<{0}>: {1}", peer_addrs[i], m.content);
                            },
                            Some(packet::v1::packet::Packet::PktCrypto(_)) => {},
                            Some(packet::v1::packet::Packet::PktFirst(_)) => {}
                            None => {}
                        }
                    },
                    Err(_) => {}
                };
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });
    loop {
         let mut input = String::new();
         let mut pkt = ProtocolPacket::default();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim();
                let message = messages::v1::Message {
                    id: "test-id".to_string(),
                    channel_id: "test-channel".to_string(),
                    username: "test-username".to_string(),
                    content: trimmed.to_string(),
                    attachments: vec![]
                };
                pkt.packet = Some(packet::v1::packet::Packet::PktMessage(message));
                for app_outbound_tx in &senders {
                    let _ = app_outbound_tx.send(pkt.clone()).await;
                }
            }
            Err(_) => {}
        }
    }
}
