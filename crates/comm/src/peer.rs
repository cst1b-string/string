//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{cmp::Reverse, net::SocketAddr, sync::Arc, time::Duration};

use protocol::{try_decode_packet, try_encode_packet, ProtocolPacket, packet};
use thiserror::Error;
use tokio::sync::{
    mpsc::{self, error::SendError},
    RwLock,
};
use tracing::{debug, span, trace, warn, Level};

use crate::socket::{
    SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
};
use crate::crypto::Crypto;

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
    // This object will handle the key exchange and encryption needs
    pub crypto: Arc<RwLock<Crypto>>,
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

        // channel for sending and receiving SocketPackets to/from the network
        let (net_inbound_tx, net_inbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (net_outbound_tx, net_outbound_rx) = mpsc::channel(CHANNEL_SIZE);

        // shared state
        let state = Arc::new(RwLock::new(match initiate {
            true => PeerState::Init,
            false => PeerState::Connect,
        }));

        let crypto = Arc::new(RwLock::new(Crypto::new()));

        span!(Level::TRACE, "peer::receiver", %remote_addr).in_scope(|| {
            start_receiver_worker(
                state.clone(),
                net_inbound_rx,
                net_outbound_tx.clone(),
                app_inbound_tx.clone(),
                crypto.clone()
            )
        });

        span!(Level::TRACE, "peer::sender", %remote_addr).in_scope(|| {
            start_sender_worker(state.clone(), app_outbound_rx, net_outbound_tx.clone(), crypto.clone())
        });

        (
            Self {
                remote_addr,
                app_outbound_tx,
                net_inbound_tx,
                state,
                crypto
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
fn start_sender_worker(
    state: Arc<RwLock<PeerState>>,
    mut app_outbound_rx: mpsc::Receiver<ProtocolPacket>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    crypto: Arc<RwLock<Crypto>>
) {
    tokio::task::spawn(async move {
        let mut syns_sent: u32 = 0;
        loop {
            trace!("start_sender_worker loop");
            // ensure we're in a state where we can send packets
            let current_state = { *state.read().await };
            if current_state == PeerState::Dead {
                warn!("peer is dead, breaking out of sender worker loop");
                break;
            }
            // We're initiating, let's send a Syn to kickstart the process
            if current_state == PeerState::Init {
                try_break!(
                net_outbound_tx
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

            // split packet into network packets and send
            for chunk in buf
                .chunks(MAX_PROTOCOL_PACKET_CHUNK_SIZE)
            {
                let mut crypto_ = crypto.write().await;
                let actual_chunk = match crypto_.encrypt(&chunk.to_vec()) {
                    Ok(enc) => enc,
                    Err(_) => { continue; }
                };
                let net_packet = SocketPacket::new(SocketPacketType::Data, 0, 0, true, actual_chunk);
                match net_outbound_tx.send(net_packet).await {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    });
}

/// Starts the background tasks that handle receiving packets from the network and forwarding their
/// decoded contents to the application.
fn start_receiver_worker(
    state: Arc<RwLock<PeerState>>,
    mut net_inbound_rx: mpsc::Receiver<SocketPacket>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    app_inbound_tx: mpsc::Sender<protocol::packet::v1::Packet>,
    crypto: Arc<RwLock<Crypto>>
) {
    tokio::task::spawn(async move {
        // priority queue for packets - this guarantees correct sequencing of UDP
        // packets that make up a single protocol message
        let mut packet_queue = std::collections::BinaryHeap::new();

        loop {
            trace!("start_receiver_worker loop");

            // receive packet from network
            let mut packet: SocketPacket = match net_inbound_rx.recv().await {
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
                                net_outbound_tx
                                    .send(SocketPacket::new(
                                        SocketPacketType::SynAck,
                                        packet.packet_number + 1,
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

                            // A bit hackish
                            {
                                let key_init_packet = crypto.read().await.kex_packet();
                                let buf = match try_encode_packet(&key_init_packet) {
                                    Ok(buf) => buf,
                                    Err(e) => {
                                        eprintln!("Failed to encode packet: {:?}", e);
                                        continue;
                                    }
                                };
                                try_break!(
                                    net_outbound_tx
                                    .send(SocketPacket::new(
                                        SocketPacketType::Data,
                                        0,
                                        0,
                                        false,
                                        buf,
                                    ))
                                    .await
                                );
                            }
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
                            let ack = SocketPacket::new(
                                SocketPacketType::Ack,
                                packet.packet_number + 1,
                                0,
                                false,
                                vec![],
                            );
                            // write to network
                            try_break!(net_outbound_tx.send(ack).await);
                        }
                        SocketPacketType::SynAck => {
                            // transition to established state
                            debug!(
                                ?current_state,
                                next = ?PeerState::KeyRecv,
                                "state transition"
                            );
                            *state.write().await = PeerState::KeyRecv;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Established
                | PeerState::KeyRecv
                | PeerState::KeyInit
                | PeerState::AwaitFirst => match packet.packet_type {
                    SocketPacketType::Syn
                    | SocketPacketType::Ack
                    | SocketPacketType::SynAck
                    | SocketPacketType::Heartbeat
                    | SocketPacketType::Invalid => {}
                    SocketPacketType::Data => {
                        // send ack
                        try_break!(
                            net_outbound_tx
                                .send(SocketPacket::new(
                                    SocketPacketType::Ack,
                                    packet.packet_number,
                                    0,
                                    false,
                                    vec![],
                                ))
                                .await
                        );

                        if packet.encrypted {
                            let mut crypto_ = crypto.write().await;
                            match crypto_.decrypt(&packet.data) {
                                Ok(dec) => { packet.data = dec; }
                                Err(_) => { continue; }
                            }
                        }

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

                        if current_state == PeerState::KeyInit ||
                            current_state == PeerState::KeyRecv {
                            let result = crypto.write().await.handle_kex(packet, current_state);
                            match result {
                                Ok(_) => {
                                    let mut next_state = PeerState::Established;
                                    if current_state == PeerState::KeyRecv {
                                        {
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
                                        next_state = PeerState::AwaitFirst;
                                    }
                                    else {
                                        {
                                            let mut first_pkt = ProtocolPacket::default();
                                            first_pkt.packet = Some(
                                                packet::v1::packet::Packet::PktFirst(
                                                    packet::v1::FirstPacket {}
                                                )
                                            );
                                            let buf_ = match try_encode_packet(&first_pkt) {
                                                Ok(buf) => buf,
                                                Err(e) => {
                                                    eprintln!("Failed to encode packet: {:?}", e);
                                                    continue;
                                                }
                                            };
                                            let first_ = crypto.write().await.encrypt(&buf_.to_vec());
                                            match first_ {
                                                Ok(first) => {
                                                    match net_outbound_tx
                                                        .send(SocketPacket::new(
                                                            SocketPacketType::Data,
                                                            0,
                                                            0,
                                                            true,
                                                            first,
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
                                    }
                                    debug!(
                                        ?current_state,
                                        next = ?next_state,
                                        "state transition"
                                    );
                                    *state.write().await = next_state;
                                }
                                Err(_) => {}
                            }
                        }
                        else if current_state == PeerState::AwaitFirst {
                            {
                                debug!(
                                    ?current_state,
                                    next = ?PeerState::AwaitFirst,
                                    "state transition"
                                );
                                *state.write().await = PeerState::Established;
                            }
                        }
                        else if current_state == PeerState::Established {
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
