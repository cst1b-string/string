use clap::Parser;
use smallvec::*;
use std::{
    env,
    fs::File,
    io::{self, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use string_comm::Socket;
use string_protocol::{messages, AttachmentType, ProtocolPacket, ProtocolPacketType};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use pgp::{
    composed::{key::SecretKeyParamsBuilder, KeyType, SignedSecretKey},
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::{CompressionAlgorithm, KeyTrait, SecretKeyTrait},
    Deserializable,
};

use image::{guess_format, ImageFormat};

mod lighthouse;

/// comm-test is a simple tool to test the string-comm crate.
#[derive(Debug, Parser)]
struct Args {
    /// The source port to bind to.
    #[clap(long)]
    port: u16,
    /// URL of lighthouse
    #[clap(long)]
    lighthouse_url: String,
    /// Username of node, needed for cert gen
    #[clap(long)]
    username: String,
}

fn generate_key(username: String, password: String) -> SignedSecretKey {
    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
        .key_type(KeyType::Rsa(2048))
        .can_certify(false)
        .can_sign(true)
        .primary_user_id(username)
        .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256,])
        .preferred_hash_algorithms(smallvec![HashAlgorithm::SHA2_256,])
        .preferred_compression_algorithms(smallvec![CompressionAlgorithm::ZLIB,]);

    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");
    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");
    let passwd_fn = || password;
    secret_key
        .sign(passwd_fn)
        .expect("Must be able to sign its own metadata")
}

fn load_key(location: &String) -> Option<SignedSecretKey> {
    let Ok(mut file) = File::open(location) else {
        return None;
    };
    let Ok((key, _headers)) = SignedSecretKey::from_armor_single(&mut file) else {
        return None;
    };
    Some(key)
}

fn save_key(location: &String, key: SignedSecretKey) {
    let mut file = File::create(location).expect("Error opening privkey file");
    file.write_all(
        key.to_armored_string(None)
            .expect("Error generating armored string")
            .as_bytes(),
    )
    .expect("Error writing privkey");
}

fn get_key_path() -> String {
    let cwd = env::current_dir().expect("Failed to get current dir");
    let cwd_str = cwd.to_str().expect("Failed to convert dir to string");
    format!("{cwd_str}/key.asc")
}

fn construct_image(image_data: &Vec<u8>) -> messages::v1::MessageAttachment {
    let format = match guess_format(image_data.as_slice()).unwrap_or(ImageFormat::Png) {
        ImageFormat::Png => messages::v1::ImageFormat::Png,
        ImageFormat::Jpeg => messages::v1::ImageFormat::Jpeg,
        ImageFormat::Gif => messages::v1::ImageFormat::Gif,
        ImageFormat::WebP => messages::v1::ImageFormat::Webp,
        _ => messages::v1::ImageFormat::Unspecified,
    };
    messages::v1::MessageAttachment {
        attachment_type: Some(AttachmentType::Image(messages::v1::ImageAttachment {
            format: format.into(),
            data: image_data.clone(),
        })),
    }
}

fn display_attachments(username: String, attachments: Vec<messages::v1::MessageAttachment>) {
    if !attachments.is_empty() {
        info!("<{0}>: ", username);
    }
    for iter in attachments {
        if let Some(AttachmentType::Image(messages::v1::ImageAttachment { format: _, data })) =
            iter.attachment_type
        {
            if let Ok(img) = image::load_from_memory(&data) {
                let config = &artem::config::ConfigBuilder::new().build();
                let ascii = artem::convert(img, config);
                println!("{}", ascii);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let Args {
        port: bind_port,
        lighthouse_url,
        username,
    } = Args::parse();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

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
    let myfingerprint = secret_key.public_key().fingerprint();

    info!("[+] Key loaded!");

    // bind to the socket
    let socket = match Socket::bind(([0, 0, 0, 0], bind_port).into(), secret_key.clone()).await {
        Ok(s) => s,
        Err(_) => {
            error!("[-] Failed to bind to local.");
            return;
        }
    };

    let myid: String;
    let myip: Option<Ipv4Addr>;
    let myport: u16;
    if let IpAddr::V4(ipv4) = socket.external.ip() {
        myip = Some(ipv4);
        myport = socket.external.port();
        myid = lighthouse::register_endpoint(&lighthouse_url, myip, myport, secret_key.clone())
            .await
            .expect("failed to register endpoint");

        let myfingerprint_hex = hex::encode(myfingerprint.clone());
        let info_str = lighthouse::encode_info_str(&myfingerprint_hex, &lighthouse_url, &myid);

        info!("[+] Info string: {0}", info_str);
    } else {
        error!("[-] IPv6 not supported");
        return;
    }

    // add peers
    let senders: Arc<RwLock<Vec<mpsc::Sender<ProtocolPacket>>>> = Arc::new(RwLock::new(Vec::new()));
    let receivers: Arc<RwLock<Vec<mpsc::Receiver<ProtocolPacket>>>> =
        Arc::new(RwLock::new(Vec::new()));

    let senders_1 = senders.clone();
    let receivers_1 = receivers.clone();

    let senders_2 = senders.clone();
    let receivers_2 = receivers.clone();

    let socket_locked = Arc::new(RwLock::new(socket));
    let socket_locked_1 = socket_locked.clone();

    info!("[+] Use /conn <info string> to connect to a new peer");
    info!("[+] Use /dr <username> to start a chat with user");
    info!("[+] Then use /msg <username> <message> to send a message");
    info!("[+] Then use /msgimg <username> <image path> to send an image");
    info!("[+] Chat log follows below:");

    tokio::task::spawn(async move {
        loop {
            for app_inbound_rx in receivers.write().await.iter_mut() {
                if let Ok(recv) = app_inbound_rx.try_recv() {
                    match recv.packet_type {
                        Some(ProtocolPacketType::PktMessage(m)) => {
                            info!("<{0}>: {1}", m.username.clone(), m.content);
                            display_attachments(m.username, m.attachments);
                        }
                        Some(_) => {}
                        None => {}
                    }
                };
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });

    tokio::task::spawn(async move {
        let mut seen: Vec<String> = Vec::new();
        loop {
            let conns = lighthouse::list_conns(&lighthouse_url, myid.clone(), secret_key.clone())
                .await
                .expect("list conn failed");
            for (conn, fingerprint) in conns.iter() {
                if !seen.contains(conn) {
                    info!("[*] New connection from {:?}", conn);
                    let (app_outbound_tx, app_inbound_rx) = socket_locked
                        .write()
                        .await
                        .add_peer(
                            SocketAddr::from_str(conn).expect("bad conn"),
                            hex::decode(fingerprint).expect("bad fingerprint"),
                            false,
                        )
                        .await;
                    {
                        senders_1.write().await.push(app_outbound_tx);
                        receivers_1.write().await.push(app_inbound_rx);
                    }
                    seen.push(conn.clone());
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
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
                        let _ = socket_locked_1
                            .write()
                            .await
                            .start_dr(rest.to_string())
                            .await;
                        info!("[+] Done DR with {0}", rest.to_string());
                    } else if prefix == "cert" {
                        let _ = socket_locked_1
                            .write()
                            .await
                            .get_node_cert(rest.to_string())
                            .await;
                    } else if prefix == "msg" {
                        if let Some((destination, message)) = rest.split_once(' ') {
                            let message = messages::v1::Message {
                                id: "test-id".to_string(),
                                channel_id: "test-channel".to_string(),
                                username: username.to_string(),
                                content: message.to_string(),
                                attachments: vec![],
                                time_sent: None,
                            };
                            let packet = ProtocolPacket {
                                packet_type: Some(ProtocolPacketType::PktMessage(message)),
                            };
                            let _ = socket_locked_1
                                .write()
                                .await
                                .send_gossip_encrypted(packet, destination.to_string())
                                .await;
                        }
                    } else if prefix == "msgimg" {
                        if let Some((destination, image_path)) = rest.split_once(' ') {
                            if let Ok(mut image_file) = File::open(image_path) {
                                let mut image_data = Vec::new();
                                if image_file.read_to_end(&mut image_data).is_ok() {
                                    let img = construct_image(&image_data);
                                    let message = messages::v1::Message {
                                        id: "test-id".to_string(),
                                        channel_id: "test-channel".to_string(),
                                        username: username.to_string(),
                                        content: "".to_string(),
                                        attachments: vec![img],
                                        time_sent: None,
                                    };
                                    let packet = ProtocolPacket {
                                        packet_type: Some(ProtocolPacketType::PktMessage(message)),
                                    };
                                    let _ = socket_locked_1
                                        .write()
                                        .await
                                        .send_gossip_encrypted(packet, destination.to_string())
                                        .await;
                                }
                            }
                        }
                    } else if prefix == "conn" {
                        let (fingerprint, target_lh_url, id) =
                            lighthouse::decode_info_str(&rest.to_string())
                                .expect("bad info string");
                        let target = lighthouse::lookup_endpoint(
                            &target_lh_url,
                            id,
                            myip,
                            myport,
                            &myfingerprint,
                        )
                        .await
                        .expect("failed to lookup endpoint");
                        let (app_outbound_tx, app_inbound_rx) = socket_locked_1
                            .write()
                            .await
                            .add_peer(
                                SocketAddr::from_str(&target).expect("bad target"),
                                hex::decode(fingerprint).expect("bad fingerprint"),
                                true,
                            )
                            .await;
                        info!("[+] Sent request to {:?}", target);
                        {
                            senders_2.write().await.push(app_outbound_tx);
                            receivers_2.write().await.push(app_inbound_rx);
                        }
                    }
                }
            }
        }
    }
}
