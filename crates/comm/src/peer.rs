//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{cmp::Reverse, collections::HashMap, net::SocketAddr, sync::Arc};

use protocol::{packet, try_decode_packet, try_encode_packet, ProtocolPacket};
use thiserror::Error;
use tokio::select;
use tokio::sync::{
    mpsc::{self, error::SendError},
    Mutex, RwLock,
};
use tokio::time::{sleep, Duration, Instant};

use tracing::{debug, span, trace, warn, Level};

use crate::crypto::Crypto;
use crate::socket::{
    SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
};

/// A convenient macro for breaking out of a loop if an error occurs.
macro_rules! try_break {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(_) => break,
        }
    };
}

/// The buffer size of the various channels used for passing data between the network tasks.
const CHANNEL_SIZE: usize = 32;

/// The maximum size of an [ProtocolPacket] chunk before it needs to be split into multiple
/// [SocketPacket]s.
const MAX_PROTOCOL_PACKET_CHUNK_SIZE: usize = UDP_MAX_DATAGRAM_SIZE - MIN_SOCKET_PACKET_SIZE;

/// The state of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Init,
    Connect,
    KeyInit,
    KeyRecv,
    AwaitFirst,
    Established,
    Dead,
}

/// Represents a connection to a remote peer. All communication between peers is done using a
/// shared UDP socket. Implements a simple state machine (SM) to manage the connection.
///
/// Makes use of four [tokio::sync::mpsc] channels:
/// - `app_outbound_tx` is used to send [ProtocolPacket]s from the application to the peer SM.
/// - `app_inbound_rx` is used to receive [ProtocolPacket]s from the peer SM to the application.
/// - `net_outbound_tx` is used to send [SocketPacket]s from the peer SM to the network.
/// - `net_inbound_rx` is used to receive [SocketPacket]s from the network to the peer SM.
#[derive(Debug)]
pub struct Peer {
    /// The destination address.
    pub remote_addr: SocketAddr,
    /// The inbound [ProtocolPacket] channel. This is used to receive packets from the application.
    pub app_outbound_tx: mpsc::Sender<ProtocolPacket>,
    /// The inbound [SocketPacket] channel. This is used to receive packets from the network.
    pub net_inbound_tx: mpsc::Sender<SocketPacket>,
    pub state: Arc<RwLock<PeerState>>,
    /// This object will handle the key exchange and encryption needs
    pub crypto: Arc<RwLock<Crypto>>,
    /// packet number is the value of the last received/transmitted packet + 1 for data packets
    /// for Ack/SynAck etc, its the same as the last received/transmitted data packet (so the sender knows what packet you're replying to)
    /// TODO: unsure about kex
    pub packet_number: Arc<Mutex<u32>>,
    /// keep a hashmap of ((packet_number, chunk_number), SocketPacket) to retransmit in case there's no Ack
    /// There might be some redundancy here? i.e., we could just recreate store the fields to
    /// recreate the SocketPacket but probably shouldn't be too much of an issue for now
    pub packets_being_sent: Arc<RwLock<HashMap<(u32, u32), SocketPacket>>>,
}

/// An enumeration of possible errors that can occur when working with peers.
#[derive(Error, Debug)]
pub enum PeerError {
    // Failed to send packet between threads.
    #[error("Failed to send packet to network thread")]
    NetworkSendFail(#[from] SendError<SocketPacket>),
    // Failed to send packet between threads.
    #[error("Failed to send packet to application")]
    ApplicationSendFail(#[from] SendError<ProtocolPacket>),
}

impl Peer {
    /// Create a new connection to the given destination.
    #[tracing::instrument(name = "peer", skip(initiate))]
    pub fn new(
        remote_addr: SocketAddr,
        initiate: bool,
    ) -> (
        Self,
        mpsc::Receiver<ProtocolPacket>,
        mpsc::Receiver<SocketPacket>,
    ) {
        // channels for sending and receiving ProtocolPackets to/from the application
        let (app_inbound_tx, app_inbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (app_outbound_tx, app_outbound_rx) = mpsc::channel(CHANNEL_SIZE);

        let (peer_inbound_tx, peer_inbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (peer_outbound_tx, peer_outbound_rx) = mpsc::channel(CHANNEL_SIZE);

        // channel for sending and receiving SocketPackets to/from the network
        let (net_inbound_tx, net_inbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (net_outbound_tx, net_outbound_rx) = mpsc::channel(CHANNEL_SIZE);

        // shared state
        let state = Arc::new(RwLock::new(match initiate {
            true => PeerState::Init,
            false => PeerState::Connect,
        }));

        let crypto = Arc::new(RwLock::new(Crypto::new()));
        let packet_number = Arc::new(Mutex::new(0));
        let packets_being_sent = Arc::new(RwLock::new(HashMap::new()));

        span!(Level::TRACE, "peer::receiver", %remote_addr).in_scope(|| {
            start_peer_receiver_worker(
                state.clone(),
                peer_outbound_tx.clone(),
                app_inbound_tx.clone(),
                peer_inbound_rx,
                crypto.clone(),
                packet_number.clone(),
                packets_being_sent.clone(),
            )
        });

        span!(Level::TRACE, "peer::sender", %remote_addr).in_scope(|| {
            start_peer_sender_worker(
                state.clone(),
                peer_outbound_tx.clone(),
                app_inbound_tx.clone(),
                app_outbound_rx,
                crypto.clone(),
                packet_number.clone(),
                packets_being_sent.clone(),
            )
        });

        span!(Level::TRACE, "crypto::receiver", %remote_addr).in_scope(|| {
            start_crypto_receiver_worker(
                state.clone(),
                net_outbound_tx.clone(),
                peer_inbound_tx.clone(),
                net_inbound_rx,
                peer_outbound_tx.clone(), // For sending first packet
                crypto.clone(),
            )
        });

        span!(Level::TRACE, "crypto::sender", %remote_addr).in_scope(|| {
            start_crypto_sender_worker(
                state.clone(),
                net_outbound_tx.clone(),
                peer_inbound_tx.clone(),
                peer_outbound_rx,
                crypto.clone(),
            )
        });

        (
            Self {
                remote_addr,
                app_outbound_tx,
                net_inbound_tx,
                state,
                crypto,
                packet_number,
                packets_being_sent,
            },
            app_inbound_rx,
            net_outbound_rx,
        )
    }

    /// Send a packet to the peer.
    pub async fn send_packet(&mut self, packet: ProtocolPacket) -> Result<(), PeerError> {
        self.app_outbound_tx
            .send(packet)
            .await
            .map_err(PeerError::ApplicationSendFail)?;
        Ok(())
    }
}

/// Starts the background task that handles sending packets to the network, taking
/// packets from the application, encoding them as [NetworkPacket]s, before sending them to the network.
fn start_peer_sender_worker(
    state: Arc<RwLock<PeerState>>,
    peer_outbound_tx: mpsc::Sender<SocketPacket>,
    app_inbound_tx: mpsc::Sender<ProtocolPacket>,
    mut app_outbound_rx: mpsc::Receiver<ProtocolPacket>,
    crypto: Arc<RwLock<Crypto>>,
    packet_number: Arc<Mutex<u32>>,
    packets_being_sent: Arc<RwLock<HashMap<(u32, u32), SocketPacket>>>,
) {
    tokio::task::spawn(async move {
        let mut syns_sent: u32 = 0;
        loop {
            trace!("start_peer_sender_worker loop");
            // ensure we're in a state where we can send packets
            let current_state = { *state.read().await };
            if current_state == PeerState::Dead {
                warn!("peer is dead, breaking out of sender worker loop");
                break;
            }
            // Send syn regardless of which end we are
            // Only the receiving side will acknowledge
            if current_state == PeerState::Init || current_state == PeerState::Connect {
                try_break!(
                    peer_outbound_tx
                        .send(SocketPacket::new(
                            SocketPacketType::Syn,
                            syns_sent,
                            0,
                            false,
                            vec![],
                        ))
                        .await
                );
                syns_sent += 1;
            }
            if current_state != PeerState::Established {
                debug!("peer is not established, sleeping for 500ms");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            // receive packet from queue
            trace!("receive packet from queue");
            let packet: ProtocolPacket = match app_outbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            // encode packet
            trace!("encode packet: {:?}", packet);
            let buf = match try_encode_packet(&packet) {
                Ok(buf) => buf,
                Err(e) => {
                    eprintln!("Failed to encode packet: {:?}", e);
                    continue;
                }
            };

            let mut packet_number = packet_number.lock().await;
            let mut packets = packets_being_sent.write().await;

            // split packet into network packets and send
            for net_packet in
                buf.chunks(MAX_PROTOCOL_PACKET_CHUNK_SIZE)
                    .enumerate()
                    .map(|(chunk_idx, chunk)| {
                        SocketPacket::new(
                            SocketPacketType::Data,
                            *packet_number,
                            chunk_idx as u32,
                            false,
                            chunk,
                        )
                    })
            {
                match peer_outbound_tx.send(net_packet.clone()).await {
                    Ok(_) => {
                        // add the packet to hashmap of packets that we don't have a ACK to
                        packets.insert(
                            (net_packet.packet_number, net_packet.chunk_number),
                            net_packet.clone(),
                        );
                        let packets_being_sent = packets_being_sent.clone();
						let peer_outbound_tx_clone = peer_outbound_tx.clone();
						let state_clone = state.clone();

                        // spawn a new task that keeps checking if we've received an ACK yet
                        // if we haven't, resend the packet
                        tokio::spawn(async move {
                            let timeout_duration = Duration::from_secs(30);

                            let (packet_number, chunk_number) =
                                (net_packet.packet_number, net_packet.chunk_number);
                            select! {
                                _ = sleep(timeout_duration) => {
                                    debug!("packet with number {} chunk {} did not receive an ACK in 30s, changing to dead state", packet_number, chunk_number);
                                    *state_clone.write().await = PeerState::Dead;
                                },

                                _ = async {
                                    let packets = packets_being_sent.read().await;
                                    while packets.contains_key(&(packet_number, chunk_number)){
                                        debug!("packet with number {} chunk {} did not receive an ACK, sleeping for 500ms and trying again", packet_number, chunk_number);
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        match peer_outbound_tx_clone.send(net_packet.clone()).await {
                                            Ok(_) => {},
                                            Err(_) => break
                                        }
                                    }
                                } => {}
                            }
                        });
                    }
                    Err(_) => break,
                };
            }
            *packet_number += 1;
        }
    });
}

/// Starts the background tasks that handle receiving packets from the network and forwarding their
/// decoded contents to the application.
fn start_peer_receiver_worker(
    state: Arc<RwLock<PeerState>>,
    peer_outbound_tx: mpsc::Sender<SocketPacket>,
    app_inbound_tx: mpsc::Sender<ProtocolPacket>,
    mut peer_inbound_rx: mpsc::Receiver<SocketPacket>,
    crypto: Arc<RwLock<Crypto>>,
    packet_number: Arc<Mutex<u32>>,
    packets_being_sent: Arc<RwLock<HashMap<(u32, u32), SocketPacket>>>,
) {
    tokio::task::spawn(async move {
        // priority queue for packets - this guarantees correct sequencing of UDP
        // packets that make up a single protocol message
        let mut packet_queue = std::collections::BinaryHeap::new();

        loop {
            trace!("start_peer_receiver_worker loop");

            // receive packet from crypto
            let packet: SocketPacket = match peer_inbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            // read current state
            let current_state = {
                let state = *state.read().await;
                trace!(state = ?state, "acquire state read lock");
                state
            };
            debug!(
                ?current_state,
                kind = ?packet.packet_type,
                "received packet"
            );

            match current_state {
                PeerState::Init => {
                    match packet.packet_type {
                        SocketPacketType::Syn | SocketPacketType::SynAck => {
                            // initiator never receives SYN or SYNACK
                        }
                        SocketPacketType::Ack => {
                            // write to network
                            try_break!(
                                peer_outbound_tx
                                    .send(SocketPacket::new(
                                        SocketPacketType::SynAck,
                                        packet.packet_number,
                                        0,
                                        false,
                                        vec![],
                                    ))
                                    .await
                            );
                            // transition to key init state
                            debug!(
                                ?current_state,
                                next = ?PeerState::KeyInit,
                                "state transition"
                            );
                            *state.write().await = PeerState::KeyInit;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid
                        | SocketPacketType::Kex => {}
                    }
                }
                PeerState::Connect => {
                    match packet.packet_type {
                        SocketPacketType::Ack => {
                            // responder never receives ACK
                        }
                        SocketPacketType::Syn => {
                            let ack = SocketPacket::new(
                                SocketPacketType::Ack,
                                packet.packet_number,
                                0,
                                false,
                                vec![],
                            );
                            // write to network
                            try_break!(peer_outbound_tx.send(ack).await);
                        }
                        SocketPacketType::SynAck => {}
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Kex
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Established
                | PeerState::KeyRecv
                | PeerState::KeyInit
                | PeerState::AwaitFirst => match packet.packet_type {
                    SocketPacketType::Syn
                    | SocketPacketType::SynAck
                    | SocketPacketType::Heartbeat
                    | SocketPacketType::Kex
                    | SocketPacketType::Invalid => {}
                    SocketPacketType::Ack => {
                        let mut packets = packets_being_sent.write().await;
                        packets.remove(&(packet.packet_number, packet.chunk_number));
                    }
                    SocketPacketType::Data => {
                        // send ack
                        try_break!(
                            peer_outbound_tx
                                .send(SocketPacket::new(
                                    SocketPacketType::Ack,
                                    packet.packet_number,
                                    packet.chunk_number,
                                    false,
                                    vec![],
                                ))
                                .await
                        );

                        // add packet to queue
                        packet_queue.push(Reverse(packet));

                        // attempt to decode
                        let data_len: usize = packet_queue
                            .iter()
                            .map(|Reverse(packet)| packet.data.len())
                            .sum();

                        let mut buf = Vec::with_capacity(data_len);

                        packet_queue
                            .iter()
                            .for_each(|Reverse(packet)| buf.append(&mut packet.data.clone()));

                        let packet = match try_decode_packet(buf) {
                            Ok(packet) => packet,
                            Err(_) => continue,
                        };

                        if current_state == PeerState::Established {
                            // forward to application
                            debug!(?packet, "forward packet to application");
                            try_break!(app_inbound_tx.send(packet).await);

                            // clear queue
                            debug!("clear packet queue");
                            packet_queue.clear();
                        }
                    }
                },
                PeerState::Dead => {}
            }
        }
    });
}

fn start_crypto_receiver_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    peer_inbound_tx: mpsc::Sender<SocketPacket>,
    mut net_inbound_rx: mpsc::Receiver<SocketPacket>,
    peer_outbound_tx: mpsc::Sender<SocketPacket>,
    crypto: Arc<RwLock<Crypto>>,
) {
    tokio::task::spawn(async move {
        loop {
            let mut packet: SocketPacket = match net_inbound_rx.recv().await {
                Some(packet) => packet,
                None => {
                    continue;
                }
            };

            let current_state = { *state.read().await };
            debug!(
                state = ?current_state,
                packet = ?packet.packet_type,
                "crypto received packet"
            );
            // Should be pass this packet to peer?
            let mut pass_packet = true;
            match packet.packet_type {
                SocketPacketType::Kex => {
                    let packet_ = match try_decode_packet(packet.data.clone()) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    match current_state {
                        PeerState::KeyRecv => {
                            // We are "Bob", or the passive receiving side.
                            // On kex packet, we send our keys to "Alice", and then
                            // wait for the compulsory first message from Alice to kickstart DR
                            let result = crypto.write().await.handle_kex(packet_, current_state);
                            match result {
                                Ok(_) => {
                                    let key_recv_packet = crypto.read().await.kex_packet();
                                    let buf = match try_encode_packet(&key_recv_packet) {
                                        Ok(buf) => buf,
                                        Err(e) => {
                                            eprintln!("Failed to encode packet: {:?}", e);
                                            continue;
                                        }
                                    };
                                    match net_outbound_tx
                                        .send(SocketPacket::new(
                                            SocketPacketType::Kex,
                                            0,
                                            0,
                                            false,
                                            buf,
                                        ))
                                        .await
                                    {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                    {
                                        *state.write().await = PeerState::AwaitFirst
                                    };
                                    pass_packet = false;
                                }
                                Err(_) => {}
                            }
                        }
                        PeerState::KeyInit => {
                            // We are "Alice", or the active sending side.
                            // We have received our reply from bob,
                            // now we encrypt an empty first message to send to bob
                            let result = crypto.write().await.handle_kex(packet_, current_state);
                            match result {
                                Ok(_) => {
                                    let mut first_pkt = ProtocolPacket::default();
                                    first_pkt.packet = Some(packet::v1::packet::Packet::PktFirst(
                                        packet::v1::FirstPacket {},
                                    ));
                                    let buf = match try_encode_packet(&first_pkt) {
                                        Ok(buf_) => buf_,
                                        Err(e) => {
                                            eprintln!("Failed to encode packet: {:?}", e);
                                            continue;
                                        }
                                    };
                                    debug!(
                                        ?current_state,
                                        next = ?PeerState::Established,
                                        "state transition"
                                    );
                                    *state.write().await = PeerState::Established;
                                    match peer_outbound_tx
                                        .send(SocketPacket::new(
                                            SocketPacketType::Data,
                                            0,
                                            0,
                                            false,
                                            buf,
                                        ))
                                        .await
                                    {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                        _ => {}
                    }
                }
                SocketPacketType::Data => {
                    match current_state {
                        PeerState::AwaitFirst => {
                            // We are "Bob", we have received our first empty encrypted message
                            // from "Alice", so we decrypt it but do not pass on to app
                            // because it's not a real message
                            debug!(
                                ?current_state,
                                next = ?PeerState::AwaitFirst,
                                "state transition"
                            );
                            {
                                *state.write().await = PeerState::Established;
                            }
                            pass_packet = false;
                        }
                        _ => {}
                    }
                }
                SocketPacketType::SynAck => {
                    // I am "Bob", ready to receive key from "Alice"
                    // We are supposed to do this in the peer receiver thread
                    // But due to code below in the sender to send a SynAck and a Kex
                    // in rapid succession, by the time the peer transitions state
                    // it would be too late
                    debug!(
                        ?current_state,
                        next = ?PeerState::KeyRecv,
                        "state transition"
                    );
                    *state.write().await = PeerState::KeyRecv;
                }
                _ => {}
            };
            if packet.encrypted {
                let mut crypto_ = crypto.write().await;
                match crypto_.decrypt(&packet.data) {
                    Ok(dec) => {
                        packet.data = dec;
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
            if pass_packet {
                match peer_inbound_tx.send(packet).await {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    });
}

fn start_crypto_sender_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    peer_inbound_tx: mpsc::Sender<SocketPacket>,
    mut peer_outbound_rx: mpsc::Receiver<SocketPacket>,
    crypto: Arc<RwLock<Crypto>>,
) {
    tokio::task::spawn(async move {
        loop {
            let packet: SocketPacket = match peer_outbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            let current_state = { *state.read().await };

            // Usually we just need to forward the one packet from peer to net
            // However, when we see a SynAck sent, we are initiating side ("Alice")
            // We should follow the SynAck with a Kex, which will be pushed into this queue
            let mut packet_queue: Vec<SocketPacket> = Vec::new();
            let packet_data: Vec<u8> = packet.data.clone();
            let packet_type = packet.packet_type.clone();
            packet_queue.push(packet);

            match packet_type {
                SocketPacketType::Data => {
                    match current_state {
                        PeerState::Established => {
                            // Encrypt data packet before sending
                            let mut crypto_ = crypto.write().await;
                            let actual = match crypto_.encrypt(&packet_data) {
                                Ok(enc) => enc,
                                Err(_) => {
                                    continue;
                                }
                            };
                            let _ = packet_queue.pop();
                            packet_queue.push(SocketPacket::new(
                                SocketPacketType::Data,
                                0,
                                0,
                                true,
                                actual,
                            ));
                        }
                        _ => {}
                    }
                }
                SocketPacketType::SynAck => {
                    let key_init_packet = crypto.read().await.kex_packet();
                    let buf = match try_encode_packet(&key_init_packet) {
                        Ok(buf) => buf,
                        Err(e) => {
                            eprintln!("Failed to encode packet: {:?}", e);
                            continue;
                        }
                    };
                    packet_queue.push(SocketPacket::new(SocketPacketType::Kex, 0, 0, false, buf));
                }
                _ => {}
            };
            for queued_packet in packet_queue {
                match net_outbound_tx.send(queued_packet).await {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    });
}
