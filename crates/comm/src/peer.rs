//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{cmp::Reverse, net::SocketAddr, sync::Arc, time::Duration};

use protocol::{packet::v1::Packet as ProtocolPacket, try_decode_packet};
use tokio::sync::{mpsc, RwLock};

use crate::{
    error::PeerError,
    packet::{NetworkPacket, NetworkPacketType},
};

const CHANNEL_SIZE: usize = 32;

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
/// - `net_outbound_tx` is used to send [NetworkPacket]s from the peer SM to the network.
/// - `net_inbound_rx` is used to receive [NetworkPacket]s from the network to the peer SM.
#[derive(Debug)]
pub struct Peer {
    /// The destination address.
    pub destination: SocketAddr,
    /// The inbound [Packet] channel. This is used to receive packets from the application.
    pub app_outbound_tx: mpsc::Sender<ProtocolPacket>,
    /// The inbound [NetworkPacket] channel. This is used to receive packets from the network.
    pub net_inbound_tx: mpsc::Sender<NetworkPacket>,
}

impl Peer {
    /// Create a new connection to the given destination.
    pub fn new(
        destination: SocketAddr,
        initiate: bool,
    ) -> (
        Self,
        mpsc::Receiver<ProtocolPacket>,
        mpsc::Receiver<NetworkPacket>,
    ) {
        // channels for sending and receiving Packets to/from the application
        let (app_inbound_tx, app_inbound_rx) = mpsc::channel(CHANNEL_SIZE);
        let (app_outbound_tx, app_outbound_rx) = mpsc::channel(CHANNEL_SIZE);

        // channel for sending and receiving NetworkPackets to/from the network
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
    net_outbound_tx: mpsc::Sender<NetworkPacket>,
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

            // split packet into network packets

            // TODO
            let data: Vec<u8> = vec![];
            let data_length: u32 = 0;

            // write to network
            let net_packet = NetworkPacket {
                packet_type: NetworkPacketType::Data,
                seq_number: 0,
                data,
                data_length: data_length as u32,
            };
            match net_outbound_tx.send(net_packet).await {
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

/// Starts the background tasks that handle receiving packets from the network and forwarding their
/// decoded contents to the application.
fn start_receiver_worker(
    state: Arc<RwLock<PeerState>>,
    mut net_inbound_rx: mpsc::Receiver<NetworkPacket>,
    net_outbound_tx: mpsc::Sender<NetworkPacket>,
    app_inbound_tx: mpsc::Sender<protocol::packet::v1::Packet>,
) {
    tokio::task::spawn(async move {
        // priority queue for packets - this guarantees correct sequencing of UDP
        // packets that make up a single protocol message
        let mut packet_queue = std::collections::BinaryHeap::new();

        loop {
            // receive packet from network
            let packet: NetworkPacket = match net_inbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            match { *state.read().await } {
                PeerState::Init => {
                    match packet.packet_type {
                        NetworkPacketType::Syn | NetworkPacketType::SynAck => {
                            // initiator never receives SYN or SYNACK
                        }
                        NetworkPacketType::Ack => {
                            let synack = NetworkPacket {
                                packet_type: NetworkPacketType::SynAck,
                                seq_number: packet.seq_number + 1,
                                data: vec![],
                                data_length: 0,
                            };
                            // write to network
                            match net_outbound_tx.send(synack).await {
                                Ok(_) => {}
                                Err(_) => break,
                            };
                            // transition to established state
                            *state.write().await = PeerState::Established;
                        }
                        NetworkPacketType::Heartbeat
                        | NetworkPacketType::Data
                        | NetworkPacketType::Invalid => {}
                    }
                }
                PeerState::Connect => {
                    match packet.packet_type {
                        NetworkPacketType::Ack => {
                            // responder never receives ACK
                        }
                        NetworkPacketType::Syn => {
                            let ack = NetworkPacket {
                                packet_type: NetworkPacketType::Ack,
                                seq_number: packet.seq_number + 1,
                                data: vec![],
                                data_length: 0,
                            };
                            // write to network
                            match net_outbound_tx.send(ack).await {
                                Ok(_) => {}
                                Err(_) => break,
                            }
                        }
                        NetworkPacketType::SynAck => {
                            // transition to established state
                            *state.write().await = PeerState::Established;
                        }
                        NetworkPacketType::Heartbeat
                        | NetworkPacketType::Data
                        | NetworkPacketType::Invalid => {}
                    }
                }
                PeerState::Established => match packet.packet_type {
                    NetworkPacketType::Syn
                    | NetworkPacketType::Ack
                    | NetworkPacketType::SynAck
                    | NetworkPacketType::Heartbeat
                    | NetworkPacketType::Invalid => {}
                    NetworkPacketType::Data => {
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
