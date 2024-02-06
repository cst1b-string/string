//! This module defines `Connection`, which manages the passing of data between two peers.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};

use crate::{
    error::PacketError,
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
    ) -> (Self, mpsc::Receiver<NetworkPacket>) {
        // channels for sending and receiving packets
        let (inbound_packet_tx, inbound_packet_rx) = mpsc::channel(32);
        let (outbound_packet_tx, outbound_packet_rx) = mpsc::channel(32);

        // state
        let state = Arc::new(RwLock::new(PeerState::Init));

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
                        // handle initial packets
                        match packet.packet_type {
                            NetworkPacketType::Syn => todo!(),
                            NetworkPacketType::SynAck => todo!(),
                            NetworkPacketType::Ack => todo!(),
                            NetworkPacketType::Heartbeat => todo!(),
                            NetworkPacketType::Data => todo!(),
                            NetworkPacketType::Invalid => todo!(),
                        }
                    }
                    PeerState::Connect => todo!(),
                    PeerState::Established => todo!(),
                    PeerState::Dead => todo!(),
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
    pub fn send_packet(&mut self, packet: NetworkPacket) -> Result<(), PacketError> {
        self.inbound_packet_tx.send(packet).await
    }

    pub fn connect(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            PeerState::Init => {
                // This cycle moves in 500 ms ticks
                let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

                let mut seqno: u64 = 0;
                let mut seen_seqno: u64 = 0;
                let mut payload = Packet::new(NetworkPacketType::Syn, seqno, None);
                let mut recv: [u8; 1024] = [0; 1024];

                while seqno < 5 * 60 * 2 {
                    let _ = socket.send_to(
                        payload.bytes(),
                        (self.destination_addr.borrow(), self.dst_port),
                    );
                    let received = socket.recv_from(&mut recv);

                    if (received.is_err()) {
                        continue;
                    }

                    match socket.recv_from(&mut recv) {
                        Ok((size, _)) => {
                            let pkt_: Option<Packet> = match Packet::from_bytes(&recv[..size]) {
                                Ok(p) => Some(p),
                                Err(_) => None,
                            };
                            match pkt_ {
                                Some(pkt) => {
                                    if self.initiate {
                                        // If initiating, wait for ACKs
                                        // On ACK send single SYNACK back
                                        match pkt.packet_type {
                                            NetworkPacketType::Ack => {
                                                // Acknowledge something we sent
                                                if pkt.sequence_number <= seqno {
                                                    let mut newpkt = Packet::new(
                                                        NetworkPacketType::SynAck,
                                                        pkt.sequence_number,
                                                        None,
                                                    );
                                                    _ = socket.send_to(
                                                        newpkt.bytes(),
                                                        (
                                                            self.destination_addr.borrow(),
                                                            self.dst_port,
                                                        ),
                                                    );
                                                    self.socket = Some(socket);
                                                    self.state = PeerState::Established;
                                                    return Ok(());
                                                }
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        // If receiving side reply ACK to every SYN
                                        // On receive SYNACK treat connection as established
                                        match pkt.packet_type {
                                            NetworkPacketType::Syn => {
                                                let mut newpkt = Packet::new(
                                                    NetworkPacketType::Ack,
                                                    pkt.sequence_number,
                                                    None,
                                                );
                                                _ = socket.send_to(
                                                    newpkt.bytes(),
                                                    (self.destination_addr.borrow(), self.dst_port),
                                                );
                                                if pkt.sequence_number > seen_seqno {
                                                    seen_seqno = pkt.sequence_number;
                                                }
                                            }
                                            NetworkPacketType::SynAck => {
                                                // Something we've seen before
                                                if pkt.sequence_number <= seen_seqno {
                                                    self.socket = Some(socket);
                                                    self.state = PeerState::Established;
                                                    return Ok(());
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                None => {}
                            };
                        }
                        Err(_) => {}
                    }
                    seqno += 1;
                    payload = Packet::new(NetworkPacketType::Syn, seqno, None);
                    thread::sleep(Duration::from_millis(500));
                }
                Err(ConnectionError::ConnTimeout)
            }
            PeerState::Established => {
                // Already connected
                Err(ConnectionError::ConnExists)
            }
        }
    }

    pub fn heartbeat(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            PeerState::Established => {
                match &self.socket {
                    Some(socket) => {
                        let mut pkt = Packet::new(NetworkPacketType::Heartbeat, 0, None);
                        let _ = socket
                            .send_to(pkt.bytes(), (self.destination_addr.borrow(), self.dst_port));
                    }
                    None => {
                        return Err(ConnectionError::Unknown);
                    }
                }
                Ok(())
            }
            PeerState::Init => Err(ConnectionError::ConnDead),
        }
    }
}
