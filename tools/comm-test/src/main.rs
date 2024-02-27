use clap::Parser;
use std::{
    io::{self, Write},
    net::SocketAddr, 
    time::Duration,
    fs::File,
    env,
    path::PathBuf
};
use string_comm::{peer::PeerState, Socket};
use string_protocol::{messages, ProtocolPacket, ProtocolPacketType};
use tokio::sync::mpsc;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use smallvec::*;

use pgp::{
    composed::{
        KeyType,
        KeyDetails,
        SecretKey,
        SecretSubkey,
        key::SecretKeyParamsBuilder,
        SignedSecretKey
    },
    errors::Result,
    packet::{KeyFlags, UserAttribute, UserId},
    types::{KeyTrait, PublicKeyTrait, SecretKeyTrait, CompressionAlgorithm},
    crypto::{sym::SymmetricKeyAlgorithm, hash::HashAlgorithm},
    Deserializable
};

/// comm-test is a simple tool to test the string-comm crate.
#[derive(Debug, Parser)]
struct Args {
    /// The source port to bind to.
    #[clap(long, required = false)]
    port: Option<u16>,
    /// The destination IP address to add as a peer.
    #[clap(long, value_delimiter = ',')]
    addrs: Vec<SocketAddr>,
    /// Fingerprint string of each peer
    #[clap(value_delimiter = ',', long)]
    fingerprints: Vec<String>,
    /// Whether to initiate the connection.
    #[clap(long)]
    initiate: bool,
    #[clap(long, required = true)]
    username: String,
    #[clap(long)]
    generate: bool,
}

fn generate_key(username: String, password: String) -> SignedSecretKey {
    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
    .key_type(KeyType::Rsa(2048))
    .can_certify(false)
    .can_sign(true)
    .primary_user_id(username.into())
    .preferred_symmetric_algorithms(smallvec![
        SymmetricKeyAlgorithm::AES256,
    ])
    .preferred_hash_algorithms(smallvec![
        HashAlgorithm::SHA2_256,
    ])
    .preferred_compression_algorithms(smallvec![
        CompressionAlgorithm::ZLIB,
    ]);

    let secret_key_params = key_params.build().expect("Must be able to create secret key params");
    let secret_key = secret_key_params.generate().expect("Failed to generate a plain key.");
    let passwd_fn = || password;
    let signed_secret_key = secret_key.sign(passwd_fn).expect("Must be able to sign its own metadata");
    signed_secret_key
}

fn load_key(location: &String) -> Option<SignedSecretKey> {
    let Ok(mut file) = File::open(&location) else { return None; };
    let Ok((key, _headers)) = SignedSecretKey::from_armor_single(&mut file) else { return None; };
    Some(key)
}

fn save_key(location: &String, key: SignedSecretKey) {
    let mut file = File::create(location).expect("Error opening privkey file");
    file.write_all(
        key.to_armored_string(None).expect("Error generating armored string").as_bytes()
    ).expect("Error writing privkey");
}

fn get_key_path() -> String {
    let cwd = env::current_dir().expect("Failed to get current dir");
    let cwd_str = cwd.to_str().expect("Failed to convert dir to string");
    format!("{cwd_str}/key.asc")
}

#[tokio::main]
async fn main() {
    let Args {
        port: bind_port,
        initiate,
        addrs: peer_addrs,
        fingerprints,
        username,
        generate
    } = Args::parse();

    // initialise tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::TRACE.into())
                .from_env_lossy(),
        )
        .init();

    let key_path = get_key_path();
    let secret_key = match load_key(&key_path) {
        Some(secret) => secret,
        None => {
            info!("[*] Key not found, generating with username {0}", username);
            let secret = generate_key(username.clone(), "testpassword".to_string());
            save_key(&key_path, secret.clone());
            secret
        }
    };

    // I am hoping by only checking < instead of !=, I can leave on extra fingerprints
    // so it's easier to type the commands when testing
    if fingerprints.len() < peer_addrs.len() {
        error!("[-] Not enough fingerprints provided.");
        return;
    }

    info!("[+] Key loaded!");
    info!("[+] Fingerprint: {0}", hex::encode(secret_key.public_key().fingerprint()));

    // Only generate key
    if generate { return; }

    // bind to the socket
    let mut socket = match Socket::bind(([0, 0, 0, 0], bind_port.unwrap()).into(), secret_key).await {
        Ok(s) => s,
        Err(_) => {
            error!("[-] Failed to bind to local.");
            return;
        }
    };

    // add peers
    let mut senders: Vec<mpsc::Sender<ProtocolPacket>> = Vec::new();
    let mut receivers: Vec<mpsc::Receiver<ProtocolPacket>> = Vec::new();

    for (i, peer_addr) in peer_addrs.iter().enumerate() {
        let fingerprint = hex::decode(&fingerprints[i]).expect("Invalid fingerprint format");
        let (app_outbound_tx, app_inbound_rx) = socket.add_peer(*peer_addr, fingerprint, initiate).await;
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
                            let _ = socket.send_gossip_encrypted(
                                packet,
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
