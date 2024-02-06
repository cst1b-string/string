//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};

use crate::{
    error::PeerError,
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
    /// The outbound packet channel. This is used to send packets to the peer.
    pub outbound_packet_tx: mpsc::Sender<NetworkPacket>,
}

impl Peer {
    /// Create a new connection to the given destination.
    pub fn new(
        socket: Arc<UdpSocket>,
        destination: SocketAddr,
        initiate: bool,
    ) -> (Self, mpsc::Receiver<NetworkPacket>) {
        // channels for sending and receiving packets
        let (inbound_packet_tx, mut inbound_packet_rx) = mpsc::channel(32);
        let (outbound_packet_tx, mut outbound_packet_rx) = mpsc::channel(32);
        let (network_packet_tx, network_packet_rx) = mpsc::channel(32);

        // state
        let state = Arc::new(RwLock::new(match initiate {
            true => PeerState::Init,
            false => PeerState::Connect,
        }));

        // peer packet receiver
        let inbound_state = state.clone();
        let receiver_inbound_packet_tx = inbound_packet_tx.clone();
        let receiver_network_packet_tx = network_packet_tx.clone();
        tokio::task::spawn(async move {
            let state = inbound_state;
            let inbound_packet_tx = receiver_inbound_packet_tx;
            let network_packet_tx = receiver_network_packet_tx;
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
                                // write to network
                                match network_packet_tx.send(synack).await {
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
                                match network_packet_tx.send(ack).await {
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
                            // forward packet to application
                            match inbound_packet_tx.send(packet).await {
                                Ok(_) => {}
                                Err(_) => break,
                            }
                        }
                    },
                    PeerState::Dead => break,
                }
            }
        });

        // peer packet sender
        let outbound_state = state.clone();
        tokio::task::spawn(async move {
            loop {
                // ensure we're in a state where we can send packets
                if { *outbound_state.read().await } != PeerState::Established {
                    tokio::time::sleep(Duration::from_millis(500)).await
                }
                // receive packet from queue
                let packet: NetworkPacket = match outbound_packet_rx.recv().await {
                    Some(packet) => packet,
                    None => break,
                };
                let buf = match packet.encode() {
                    Ok(buf) => buf,
                    Err(_) => break,
                };
                // write to network
                match socket.send_to(&buf, destination).await {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        (
            Self {
                destination,
                state: PeerState::Connect,
                seq_number: 0,
                inbound_packet_tx,
                outbound_packet_tx,
            },
            network_packet_rx,
        )
    }

    /// Send a packet to the peer.
    pub async fn send_packet(&mut self, packet: NetworkPacket) -> Result<(), PeerError> {
        self.outbound_packet_tx.send(packet).await?;
        Ok(())
    }
}
