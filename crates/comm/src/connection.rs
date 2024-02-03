//! This module defines `Connection`, which manages the passing of data between two peers.

use std::net::SocketAddr;

/// The state of a connection.
pub enum PeerState {
    Disconnected,
    Connecting,
    Connected,
}

/// Represents a connection to a remote peer. All communication between peers is done using a
/// shared UDP socket.
pub struct Peer {
    /// The destination address.
    pub destination: SocketAddr,
    /// The state of the connection.
    pub state: PeerState,
}

impl From<SocketAddr> for Peer {
    fn from(destination: SocketAddr) -> Self {
        Self {
            destination,
            state: PeerState::Disconnected,
        }
    }
}

impl Peer {
    /// Create a new connection to the given destination.
    pub fn new(destination: SocketAddr) -> Self {
        Self {
            destination,
            state: PeerState::Connecting,
        }
    }

    pub fn connect(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            PeerState::Disconnected => {
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
                                                    self.state = PeerState::Connected;
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
                                                    self.state = PeerState::Connected;
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
            PeerState::Connected => {
                // Already connected
                Err(ConnectionError::ConnExists)
            }
        }
    }

    pub fn heartbeat(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            PeerState::Connected => {
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
            PeerState::Disconnected => Err(ConnectionError::ConnDead),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum NetworkPacketType {
    /// Packets sent by the initiating peer.
    Syn,
    /// Packets sent by a receiving peer.
    Ack,
    /// Packets sent by the initiating peer after receiving an ACK. Once this is sent, the connection is established.
    SynAck,
    /// Packets sent by either peer to keep the connection alive. This is done to avoid stateful firewalls from dropping the connection.
    Heartbeat,
    /// Actual communication data
    Data,
    /// An invalid packet.
    Invalid,
}
