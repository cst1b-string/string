//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{cmp::Reverse, net::SocketAddr, sync::Arc, time::Duration};

use protocol::{try_decode_packet, try_encode_packet, ProtocolPacket};
use thiserror::Error;
use tokio::sync::{
    mpsc::{self, error::SendError},
    RwLock,
};

use crate::socket::{
    SocketPacket, SocketPacketType, MIN_SOCKET_PACKET_SIZE, UDP_MAX_DATAGRAM_SIZE,
};

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
    pub destination: SocketAddr,
    /// The inbound [ProtocolPacket] channel. This is used to receive packets from the application.
    pub app_outbound_tx: mpsc::Sender<ProtocolPacket>,
    /// The inbound [SocketPacket] channel. This is used to receive packets from the network.
    pub net_inbound_tx: mpsc::Sender<SocketPacket>,
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
    pub fn new(
        destination: SocketAddr,
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
        let (net_inbound_tx, net_outbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (net_outbound_tx, net_inbound_rx) = mpsc::channel(CHANNEL_SIZE);

        // shared state
        let state = Arc::new(RwLock::new(match initiate {
            true => PeerState::Init,
            false => PeerState::Connect,
        }));

        start_receiver_worker(
            state.clone(),
            net_inbound_rx,
            net_outbound_tx.clone(),
            app_inbound_tx.clone(),
        );
        start_sender_worker(state.clone(), app_outbound_rx, net_outbound_tx.clone());

        (
            Self {
                destination,
                app_outbound_tx,
                net_inbound_tx,
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
) {
    tokio::task::spawn(async move {
        loop {
            // ensure we're in a state where we can send packets
            if { *state.read().await } != PeerState::Established {
                tokio::time::sleep(Duration::from_millis(500)).await
            }
            // receive packet from queue
            let packet: ProtocolPacket = match app_outbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            // encode packet
            let buf = match try_encode_packet(&packet) {
                Ok(buf) => buf,
                Err(e) => {
                    eprintln!("Failed to encode packet: {:?}", e);
                    continue;
                }
            };

            // split packet into network packets and send
            for net_packet in buf
                .chunks(MAX_PROTOCOL_PACKET_CHUNK_SIZE)
                .map(|chunk| SocketPacket::new(SocketPacketType::Data, 0, 0, chunk))
            {
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
) {
    tokio::task::spawn(async move {
        // priority queue for packets - this guarantees correct sequencing of UDP
        // packets that make up a single protocol message
        let mut packet_queue = std::collections::BinaryHeap::new();

        loop {
            // receive packet from network
            let packet: SocketPacket = match net_inbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            match { *state.read().await } {
                PeerState::Init => {
                    match packet.packet_type {
                        SocketPacketType::Syn | SocketPacketType::SynAck => {
                            // initiator never receives SYN or SYNACK
                        }
                        SocketPacketType::Ack => {
                            let synack = SocketPacket::new(
                                SocketPacketType::SynAck,
                                packet.packet_number + 1,
                                0,
                                vec![],
                            );
                            // write to network
                            match net_outbound_tx.send(synack).await {
                                Ok(_) => {}
                                Err(_) => break,
                            };
                            // transition to established state
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
                            let ack = SocketPacket::new(
                                SocketPacketType::Ack,
                                packet.packet_number + 1,
                                0,
                                vec![],
                            );
                            // write to network
                            match net_outbound_tx.send(ack).await {
                                Ok(_) => {}
                                Err(_) => break,
                            }
                        }
                        SocketPacketType::SynAck => {
                            // transition to established state
                            *state.write().await = PeerState::Established;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Established => match packet.packet_type {
                    SocketPacketType::Syn
                    | SocketPacketType::Ack
                    | SocketPacketType::SynAck
                    | SocketPacketType::Heartbeat
                    | SocketPacketType::Invalid => {}
                    SocketPacketType::Data => {
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

                        // forward to application
                        match app_inbound_tx.send(packet).await {
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                },
                PeerState::Dead => {}
            }
        }
    });
}
