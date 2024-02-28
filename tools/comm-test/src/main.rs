use clap::Parser;
use std::io;
use std::{net::SocketAddr, time::Duration};
use string_comm::{peer::PeerState, Socket};
use string_protocol::{messages, ProtocolPacket, ProtocolPacketType};
use tokio::sync::mpsc;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

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
    #[clap(long)]
    username: String,
}

#[tokio::main]
async fn main() {
    let Args {
        bind_port,
        initiate,
        peer_addrs,
        username,
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
    let mut socket = match Socket::bind(([0, 0, 0, 0], bind_port).into(), username.clone()).await {
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
        let (app_outbound_tx, app_inbound_rx) = socket.add_peer(*peer_addr, initiate).await;
        senders.push(app_outbound_tx);
        receivers.push(app_inbound_rx);
    }

    info!("[+] Setup success");
    info!("[*] Attempting transmission...");

    // Wait 5 mins
    let mut ready_peers: Vec<usize> = Vec::new();

    let mut tick: u16 = 0;

    while tick < 5 * 60 * 2 && ready_peers.len() != peer_addrs.len() {
        for (i, _) in peer_addrs.iter().enumerate() {
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
    info!("[+] Chat log follows below:");
    info!("[+] Use /dr <username> to start a chat with user");
    info!("[+] Then use /msg <username> <message> to send a message");

    tokio::task::spawn(async move {
        loop {
            for app_inbound_rx in receivers.iter_mut() {
                if let Ok(recv) = app_inbound_rx.try_recv() {
                    match recv.packet_type {
                        Some(ProtocolPacketType::PktMessage(m)) => {
                            info!("<{0}>: {1}", m.username, m.content);
                        }
                        Some(_) => {}
                        None => {}
                    }
                };
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });
    loop {
        let mut input = String::new();

        if (io::stdin().read_line(&mut input)).is_ok() {
            let mut trimmed = input.trim();
            if trimmed.starts_with('/') {
                trimmed = &trimmed[1..];
                if let Some((prefix, rest)) = trimmed.split_once(' ') {
                    if prefix == "dr" {
                        let _ = socket.start_dr(rest.to_string()).await;
                    } else if prefix == "msg" {
                        if let Some((destination, message)) = rest.split_once(' ') {
                            let message = messages::v1::Message {
                                id: "test-id".to_string(),
                                channel_id: "test-channel".to_string(),
                                username: username.to_string(),
                                content: message.to_string(),
                                attachments: vec![],
                            };
                            let packet = ProtocolPacket {
                                packet_type: Some(ProtocolPacketType::PktMessage(message)),
                            };
                            let _ = Socket::send_gossip_encrypted(
                                packet,
                                socket.peers.clone(),
                                destination.to_string(),
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }
}
