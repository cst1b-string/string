//! This module defines `Connection`, which manages the passing of data between two peers.

use crate::{
    crypto::{Crypto, DoubleRatchet, DoubleRatchetError},
    maybe_break,
    socket::{
        Socket, SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
    },
    try_break, try_continue,
};
use protocol::{
    crypto, gossip, try_decode_packet, try_encode_packet, try_verify_packet_sig, MessageType,
    ProtocolPacket, ProtocolPacketType,
};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    fmt,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::{
    mpsc::{self, error::SendError},
    Mutex, RwLock,
};
use tokio::{select, time::sleep};
use tracing::{debug, error, span, trace, warn, Level};

/// The buffer size of the various channels used for passing data between the network tasks.
pub const CHANNEL_SIZE: usize = 32;

/// The maximum size of an [ProtocolPacket] chunk before it needs to be split into multiple
/// [SocketPacket]s.
const MAX_PROTOCOL_PACKET_CHUNK_SIZE: usize = UDP_MAX_DATAGRAM_SIZE - MIN_SOCKET_PACKET_SIZE;

/// The state of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Init,
    Connect,
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
pub struct Peer {
    /// The destination address.
    pub remote_addr: SocketAddr,
    /// The inbound [ProtocolPacket] channel. This is used to receive packets from the application.
    pub app_outbound_tx: mpsc::Sender<ProtocolPacket>,
    /// The inbound [SocketPacket] channel. This is used to receive packets from the network.
    pub net_inbound_tx: mpsc::Sender<SocketPacket>,
    /// The current state of the peer.
    pub state: Arc<RwLock<PeerState>>,
    /// This object will handle the key exchange and encryption needs
    pub crypto: Arc<RwLock<Crypto>>,
    /// A reference to the socket's peers.
    pub peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
    /// The username of this peer.
    /// TODO: switch to a slightly more rigorous notion of identity.
    pub username: String,
}

impl fmt::Debug for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Peer {0}>", self.remote_addr)
    }
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
    // Failed to decode decrypted packet
    #[error("Failed to decode decrypted packet")]
    DecodeFail(#[from] protocol::prost::DecodeError),
    // Failed to encode packet for encryption
    #[error("Failed to encode packet for encryption")]
    EncodeFail(#[from] protocol::prost::EncodeError),
    // Failure in double ratchet
    #[error("Failure in double ratchet")]
    DRFail(#[from] DoubleRatchetError),
}

impl Peer {
    /// Create a new connection to the given destination.
    #[tracing::instrument(name = "peer", skip(initiate))]
    pub fn new(
        remote_addr: SocketAddr,
        crypto: Arc<RwLock<Crypto>>,
        peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
        username: String,
        initiate: bool,
    ) -> (
        Self,
        mpsc::Receiver<ProtocolPacket>,
        mpsc::Receiver<SocketPacket>,
    ) {
        // channels for sending and receiving ProtocolPackets to/from the application
        let (app_inbound_tx, app_inbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (app_outbound_tx, app_outbound_rx) = mpsc::channel(CHANNEL_SIZE);

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
        let packet_acks = Arc::new(RwLock::new(HashSet::new()));

        span!(Level::TRACE, "peer::receiver", %remote_addr).in_scope(|| {
            start_peer_receiver_worker(
                state.clone(),
                net_outbound_tx.clone(),
                app_inbound_tx.clone(),
                net_inbound_rx,
                remote_addr,
                peers.clone(),
                packet_number.clone(),
                packet_acks.clone(),
            )
        });

        span!(Level::TRACE, "peer::sender", %remote_addr).in_scope(|| {
            start_peer_sender_worker(
                state.clone(),
                net_outbound_tx.clone(),
                app_outbound_rx,
                crypto.clone(),
                packet_number.clone(),
                packet_acks.clone(),
            )
        });

        (
            Self {
                remote_addr,
                app_outbound_tx,
                net_inbound_tx,
                state,
                crypto,
                peers,
                username,
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

    /// Helper function to package and sign a [MessageType] as [Gossip] packet and send to this peer only
    /// For distributing gossip check Socket class instead.
    ///
    /// TODO: Sign the gossip packet's contents with our private key
    pub async fn send_gossip_single(
        &mut self,
        message: crypto::v1::signed_packet_internal::MessageType,
        destination: String,
    ) -> Result<(), PeerError> {
        let internal = crypto::v1::SignedPacketInternal {
            destination,
            source: self.username.clone(),
            message_type: Some(message),
        };
        // TODO: Sign internal
        let gossip = ProtocolPacketType::PktGossip(gossip::v1::Gossip {
            packet: Some(crypto::v1::SignedPacket {
                signature: vec![],
                signed_data: Some(internal),
            }),
        });
        let tosend = ProtocolPacket {
            packet_type: Some(gossip),
        };
        self.send_packet(tosend.clone()).await?;
        Ok(())
    }

    /// Similar to send_gossip_single, but packages a [ProtocolPacket]
    /// as an [EncryptedPacket] in a [Gossip] Packet
    pub async fn send_gossip_single_encrypted(
        &mut self,
        packet: ProtocolPacket,
        destination: String,
    ) -> Result<(), PeerError> {
        let bytes = try_encode_packet(&packet).map_err(PeerError::EncodeFail)?;
        // encrypt message contents
        let content = {
            let mut crypto = self.crypto.write().await;
            let ratchet = crypto
                .ratchets
                .get_mut(&destination)
                .ok_or(PeerError::DRFail(DoubleRatchetError::MissingRatchet))?;
            ratchet.encrypt(&bytes).map_err(PeerError::DRFail)?
        };
        self.send_gossip_single(
            MessageType::EncryptedPacket(crypto::v1::EncryptedPacket { content }),
            destination,
        )
        .await?;
        Ok(())
    }

    /// Dispatches gossip packet based on the following logic:
    /// 1. If this packet is not intended for our node as destination, return true
    ///    so the caller can forward it on
    ///
    /// Otherwise, check the message inside the gossip packet:
    ///    2. if it's a [KeyExchange] try to establish the DR ratchet
    ///    3. if it's an [EncryptedPacket] decrypt and forward it to the app

    async fn dispatch_gossip(
        &mut self,
        signed_packet: crypto::v1::SignedPacket,
        app_inbound_tx: mpsc::Sender<ProtocolPacket>,
    ) -> Result<bool, PeerError> {
        let signed_data = signed_packet.signed_data.unwrap();
        let mut forward = false;
        if signed_data.destination == self.username {
            let source = signed_data.source;
            match signed_data.message_type {
                Some(MessageType::KeyExchange(dr)) => {
                    let mut crypto_obj = self.crypto.write().await;
                    let ratchet = crypto_obj
                        .ratchets
                        .entry(source.clone())
                        .or_insert_with(DoubleRatchet::new_responder);
                    match ratchet {
                        DoubleRatchet::Responder { .. } => {
                            if ratchet.handle_kex(dr).is_ok() {};
                            let kex = ratchet.generate_kex_message();
                            drop(crypto_obj);
                            self.send_gossip_single(MessageType::KeyExchange(kex), source)
                                .await?;
                        }
                        DoubleRatchet::Initiator { .. } => {
                            if ratchet.handle_kex(dr).is_ok() {};
                            drop(crypto_obj);
                            self.send_gossip_single_encrypted(
                                ProtocolPacket { packet_type: None },
                                source,
                            )
                            .await?;
                        }
                        DoubleRatchet::AlmostInitialized { .. }
                        | DoubleRatchet::Initialized { .. } => {
                            unreachable!()
                        }
                    }
                }
                Some(MessageType::CertExchange(_cert)) => todo!(),
                Some(MessageType::EncryptedPacket(enc)) => {
                    let bytes = {
                        let mut crypto = self.crypto.write().await;
                        let ratchet = crypto
                            .ratchets
                            .get_mut(&source)
                            .ok_or(PeerError::DRFail(DoubleRatchetError::MissingRatchet))?;
                        ratchet.decrypt(&enc.content).map_err(PeerError::DRFail)?
                    };
                    let packet = try_decode_packet(bytes).map_err(PeerError::DecodeFail)?;
                    app_inbound_tx.send(packet).await?;
                }
                None => {}
            }
        } else {
            forward = true;
        }
        Ok(forward)
    }
}

/// Starts the background task that handles sending packets to the network, taking
/// packets from the application, encoding them as [NetworkPacket]s, before sending them to the network.
fn start_peer_sender_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    mut app_outbound_rx: mpsc::Receiver<ProtocolPacket>,
    crypto: Arc<RwLock<Crypto>>,
    packet_number: Arc<Mutex<u32>>,
    packet_acks: Arc<RwLock<HashSet<(u32, u32)>>>,
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
                    net_outbound_tx
                        .send(
                            SocketPacket::new(SocketPacketType::Syn, syns_sent, 0, vec![])
                                .expect("Failed to create syn packet")
                        )
                        .await
                );
                syns_sent += 1;
            }

            // if we're not established, go around again
            if current_state != PeerState::Established {
                debug!("peer is not established, sleeping for 500ms");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            // receive packet from queue
            trace!("receive packet from queue");
            let packet: ProtocolPacket = maybe_break!(app_outbound_rx.recv().await);

            // encode packet
            trace!("encode packet: {:?}", packet);
            let buf = try_continue!(try_encode_packet(&packet), "Failed to encode packet");

            // these locks may cause some contention - investigate
            let mut packet_number = packet_number.lock().await;
            let mut packet_acks_write = packet_acks.write().await;

            // split packet into network packets and send
            for net_packet in
                buf.chunks(MAX_PROTOCOL_PACKET_CHUNK_SIZE)
                    .enumerate()
                    .map(|(chunk_idx, chunk)| {
                        SocketPacket::new(
                            SocketPacketType::Data,
                            *packet_number,
                            chunk_idx as u32,
                            vec![],
                        )
                        .expect("Failed to create data packet")
                    })
            {
                match net_outbound_tx.send(net_packet.clone()).await {
                    Ok(_) => {
                        // add the packet to hashmap of packets that we don't have a ACK to
                        packet_acks_write
                            .insert((net_packet.packet_number, net_packet.chunk_number));

                        // start a task that will wait for an ACK for this packet
                        start_ack_timeout_worker(
                            state.clone(),
                            packet_acks.clone(),
                            net_outbound_tx.clone(),
                            net_packet.clone(),
                        );
                    }
                    Err(_) => break,
                };
            }
            *packet_number += 1;
        }
    });
}

/// Periodically checks if we've received an ACK for a packet, and if not, resends the packet.
/// Times out after 30s and transitions the peer to the dead state.
fn start_ack_timeout_worker(
    state: Arc<RwLock<PeerState>>,
    packet_acks: Arc<RwLock<HashSet<(u32, u32)>>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    net_packet: SocketPacket,
) {
    // spawn a new task that keeps checking if we've received an ACK yet
    // if we haven't, resend the packet
    tokio::spawn(async move {
        let timeout = Duration::from_secs(30);
        let (packet_number, chunk_number) = (net_packet.packet_number, net_packet.chunk_number);

        select! {
            _ = sleep(timeout) => {
                debug!("packet with number {} chunk {} did not receive an ACK in 30s - peer dead", packet_number, chunk_number);
                *state.write().await = PeerState::Dead;
            },

            _ = async {
                loop {
                    // wait for 1s before checking if we've received an ACK
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    let has_packet = { packet_acks.read().await.contains(&(packet_number, chunk_number))};
                    if !has_packet {
                        break;
                    }
                    // retransmit
                    try_break!(net_outbound_tx.send(net_packet.clone()).await);
                }
            } => {}
        }
    });
}

/// Starts the background tasks that handle receiving packets from the network and forwarding their
/// decoded contents to the application.
fn start_peer_receiver_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    app_inbound_tx: mpsc::Sender<ProtocolPacket>,
    mut net_inbound_rx: mpsc::Receiver<SocketPacket>,
    remote_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
    packet_number: Arc<Mutex<u32>>,
    packet_acks: Arc<RwLock<HashSet<(u32, u32)>>>,
) {
    tokio::task::spawn(async move {
        // priority queue for packets - this guarantees correct sequencing of UDP
        // packets that make up a single protocol message
        let mut packet_queue = BinaryHeap::new();

        loop {
            trace!("start_peer_receiver_worker loop");

            let packet: SocketPacket = maybe_break!(net_inbound_rx.recv().await);

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
                                net_outbound_tx
                                    .send(SocketPacket::empty(
                                        SocketPacketType::SynAck,
                                        packet.packet_number,
                                        0,
                                    ))
                                    .await
                            );
                            // transition to key init state
                            debug!(
                                ?current_state,
                                next = ?PeerState::Established,
                                "state transition"
                            );
                            *state.write().await = PeerState::Established;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Connect => {
                    match packet.packet_type {
                        SocketPacketType::Ack => {
                            // responder never receives ACK
                        }
                        SocketPacketType::Syn => {
                            let ack =
                                SocketPacket::empty(SocketPacketType::Ack, packet.packet_number, 0);
                            // write to network
                            try_break!(net_outbound_tx.send(ack).await);
                        }
                        SocketPacketType::SynAck => {
                            debug!(
                                ?current_state,
                                next = ?PeerState::Established,
                                "state transition"
                            );
                            *state.write().await = PeerState::Established;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Established => match packet.packet_type {
                    SocketPacketType::Syn
                    | SocketPacketType::SynAck
                    | SocketPacketType::Heartbeat
                    | SocketPacketType::Invalid => {}
                    SocketPacketType::Ack => {
                        let mut packets = packet_acks.write().await;
                        packets.remove(&(packet.packet_number, packet.chunk_number));
                    }
                    SocketPacketType::Data => {
                        // send ack
                        try_break!(
                            net_outbound_tx
                                .send(SocketPacket::empty(
                                    SocketPacketType::Ack,
                                    packet.packet_number,
                                    packet.chunk_number,
                                ))
                                .await
                        );

                        // add packet to queue
                        packet_queue.push(Reverse(packet));

                        // attempt to decode
                        let data_len: usize = packet_queue
                            .iter()
                            .map(|Reverse(packet)| packet.compressed_data.len())
                            .sum();

                        let mut buf = Vec::with_capacity(data_len);

                        packet_queue.iter().for_each(|Reverse(packet)| {
                            buf.append(&mut packet.compressed_data.clone())
                        });

                        let packet = match try_decode_packet(buf) {
                            Ok(packet) => packet,
                            Err(_) => continue,
                        };

                        if current_state == PeerState::Established {
                            // clear queue - return early to avoid lots of nesting
                            debug!("clear packet queue");
                            packet_queue.clear();
                            continue;
                        }

                        match packet.packet_type {
                            Some(ProtocolPacketType::PktGossip(ref gossip)) => {
                                // check if we are missing a signed ppacket
                                if let None = gossip.packet {
                                    continue;
                                }

                                let signed_packet = gossip.packet.as_ref().unwrap();

                                let forward = {
                                    let mut peers = peers.write().await;
                                    let peer = match peers.get_mut(&remote_addr) {
                                        Some(p) => p,
                                        None => {
                                            continue;
                                        }
                                    };

                                    // Verify signature on packet
                                    let signed_packet =
                                        try_continue!(try_verify_packet_sig(&signed_packet));

                                    // Dispatch gossip to respective code if its for us...
                                    peer.dispatch_gossip(
                                        signed_packet.clone(),
                                        app_inbound_tx.clone(),
                                    )
                                    .await
                                    .unwrap()
                                };
                                // ..., otherwise, forward it on to our peers
                                if forward {
                                    drop(peers);
                                    let _ =
                                        Socket::forward_gossip(packet, peers.clone(), remote_addr)
                                            .await;
                                }
                            }
                            Some(_) => {}
                            None => {}
                        }

                        // clear queue
                        debug!("clear packet queue");
                        packet_queue.clear();
                    }
                },
                PeerState::Dead => {}
            }
        }
    });
}