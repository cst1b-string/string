//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};

use crate::{
    error::{PacketError, PeerError},
    packet::{NetworkPacket, NetworkPacketType},
};

/// The state of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Init,
    Connect,
    Established,
    Dead,
}

/// Represents a connection to a remote peer. All communication between peers is done using a
/// shared UDP socket.
#[derive(Debug)]
pub struct Peer {
    /// The destination address.
    pub destination: SocketAddr,
    /// The state of the connection.
    state: PeerState,
    /// The sequence number of the last packet sent.
    seq_number: u64,
    /// The inbound packet channel. This is used to receive packets from the peer.
    pub inbound_packet_tx: mpsc::Sender<NetworkPacket>,
}

impl Peer {
    /// Create a new connection to the given destination.
    pub fn new(
        socket: Arc<UdpSocket>,
        destination: SocketAddr,
        initiate: bool,
    ) -> (Self, mpsc::Receiver<NetworkPacket>) {
        // channels for sending and receiving packets
        let (inbound_packet_tx, inbound_packet_rx) = mpsc::channel(32);
        let (outbound_packet_tx, outbound_packet_rx) = mpsc::channel(32);

        // state
        let state = Arc::new(RwLock::new(match initiate {
            true => PeerState::Init,
            false => PeerState::Connect,
        }));

        // peer packet receiver
        let inbound_state = state.clone();
        tokio::task::spawn(async {
            let state = inbound_state;
            loop {
                // receive packet from network
                let packet: NetworkPacket = match inbound_packet_rx.recv().await {
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
                                let synack = NetworkPacket {
                                    packet_type: NetworkPacketType::SynAck,
                                    seq_number: packet.seq_number + 1,
                                    data: vec![],
                                    data_length: 0,
                                };
                            }
                            NetworkPacketType::SynAck => {
                                let ack = NetworkPacket {
                                    packet_type: NetworkPacketType::Ack,
                                    seq_number: packet.seq_number + 1,
                                    data: vec![],
                                    data_length: 0,
                                };
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
                            // forward packet to application
                            inbound_packet_tx.send(packet).await;
                        }
                    },
                    PeerState::Dead => break,
                }
            }
        });

        // peer packet sender
        let outbound_state = state.clone();
        tokio::task::spawn(async {
            loop {
                // ensure we're in a state where we can send packets
                if { *outbound_state.read().await } != PeerState::Established {
                    tokio::time::sleep(Duration::from_millis(500)).await
                }
                // receive packet from queue
                let packet = match outbound_packet_rx.recv().await {
                    Some(packet) => packet,
                    None => break,
                };
            }
        });

        (
            Self {
                destination,
                state: PeerState::Connect,
                seq_number: 0,
                inbound_packet_tx,
            },
            outbound_packet_rx,
        )
    }

    /// Send a packet to the peer.
    pub async fn send_packet(&mut self, packet: NetworkPacket) -> Result<(), PeerError> {
        self.inbound_packet_tx.send(packet).await?;
        Ok(())
    }
}
