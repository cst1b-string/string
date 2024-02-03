//! # string-comms
//!
//! This crate contains the communication code for string

use std::net::*;
use std::time::Duration;
use std::thread;
use std::borrow::Borrow;
use thiserror::Error;

pub enum ConnState {
    DISCON,
    CONNECTED,
}

pub struct Connection {
    dst_ip: String,
    dst_port: u16,
    src_port: u16,
    socket: Option<UdpSocket>,
    pub state: ConnState,
    initiate: bool              // Is this node initiating?
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PacketType {
    SYN,                    // (1) Initiating client sends this
    ACK,                    // (2) Receiving client sends this back
    SYNACK,                 // (3) Initiating client sends this
                            // Now both are connected (ignoring the Two Generals' problem)
    HEARTBEAT,              // This needs to be sent as frequently as we can
                            // so the stateful firewall doesn't drop our UDP entry
    DATA,                   // Actual communication data
    INVALID
}

pub struct Packet {
    pkttype: PacketType,
    seqno: u64,             // Currently used for syn/ack in connections only;
                            // ignored for heartbeat/data
    data: Option<Vec<u8>>,
    bytes_: Vec<u8>         // Raw bytes representation of packet
}

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Unknown error")]
    Unknown,
    #[error("Connection timed out")]
    ConnTimeout,
    #[error("Already connected")]
    ConnExists,
    #[error("Not connected")]
    ConnDead,
    #[error("Binding to UDP socket failed")]
    BindFail
//    MyErr(#[from] TheirErr),
}

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Unknown error")]
    Unknown,
    #[error("Magic number incorrect")]
    BadMagic,
    #[error("Unknown packet type")]
    BadPacketType,
    #[error("Packet too small")]
    BadSize
}

const MAGIC: u32 = 0x01020304;

impl Packet {
    pub fn new(pkttype: PacketType, seqno: u64, data: Option<Vec<u8>>) -> Self {
        Self {
            pkttype,
            seqno,
            data,
            bytes_: Vec::new()
        }
    }

    pub fn bytes(&mut self) -> &[u8] {
        if self.bytes_.is_empty() {
            self.bytes_.extend_from_slice(&MAGIC.to_be_bytes());
            let pkttype: u8 = self.pkttype as u8;
            self.bytes_.push(pkttype);
            self.bytes_.extend_from_slice(&self.seqno.to_be_bytes());
            match &self.data {
                Some(data) => {
                    self.bytes_.extend(data);
                }
                None => {}
            }
        }
        self.bytes_.as_slice()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Packet, PacketError> {
        // 4 bytes magic, 1 byte type, 8 bytes seq no.
        const MIN_PACKET_SIZE: usize = 4 + 1 + 8;
        if bytes.len() < MIN_PACKET_SIZE {
            return Err(PacketError::BadSize);
        }
        if bytes[..4] == MAGIC.to_be_bytes() {
            let pkttype: PacketType = match bytes[4] {
                0 => PacketType::SYN,
                1 => PacketType::ACK,
                2 => PacketType::SYNACK,
                3 => PacketType::HEARTBEAT,
                4 => PacketType::DATA,
                _ => { return Err(PacketError::BadPacketType); }
            };

            let seqno:u64 = u64::from_be_bytes(bytes[5..13].try_into().unwrap());
            // Additional data
            let mut data: Option<Vec<u8>> = None;
            if bytes.len() > MIN_PACKET_SIZE {
                data = Some(bytes[13..].to_vec());
            }
            return Ok(Packet::new(pkttype, seqno, data));
        }
        Err(PacketError::BadMagic)
    }
}

impl Connection {
    pub fn new(ip: &str, dst_port: u16, src_port: u16, initiate: bool) -> Self {
        Self {
            state: ConnState::DISCON,
            socket: None,
            dst_ip: ip.to_owned(),
            dst_port,
            src_port,
            initiate
        }
    }

    pub fn connect(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            ConnState::DISCON => {
                let result = UdpSocket::bind(("0.0.0.0", self.src_port));
                let socket = match result {
                    Ok(sock) => sock,
                    Err(_) => {
                        // Couldn't bind to the socket
                        return Err(ConnectionError::BindFail);
                    }
                };

                // This cycle moves in 500 ms ticks
                let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

                let mut seqno:u64 = 0;
                let mut seen_seqno:u64 = 0;
                let mut payload = Packet::new(PacketType::SYN, seqno, None);
                let mut recv: [u8; 1024] = [0; 1024];

                while seqno < 5 * 60 * 2 {
                    let _ = socket.send_to(payload.bytes(), (self.dst_ip.borrow(), self.dst_port));
                    match socket.recv_from(&mut recv) {
                        Ok((size, _)) => {
                            let pkt_: Option<Packet> = match Packet::from_bytes(&recv[..size]) {
                                Ok(p) => Some(p),
                                Err(_) => None
                            };
                            match pkt_ {
                                Some(pkt) => {
                                    if self.initiate {
                                        // If initiating, wait for ACKs
                                        // On ACK send single SYNACK back
                                        match pkt.pkttype {
                                            PacketType::ACK => {
                                                // Acknowledge something we sent
                                                if pkt.seqno <= seqno {
                                                    let mut newpkt = Packet::new(PacketType::SYNACK, pkt.seqno, None);
                                                    _ = socket.send_to(newpkt.bytes(), (self.dst_ip.borrow(), self.dst_port));
                                                    self.socket = Some(socket);
                                                    self.state = ConnState::CONNECTED;
                                                    return Ok(());
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    else {
                                        // If receiving side reply ACK to every SYN
                                        // On receive SYNACK treat connection as established
                                        match pkt.pkttype {
                                            PacketType::SYN => {
                                                let mut newpkt = Packet::new(PacketType::ACK, pkt.seqno, None);
                                                _ = socket.send_to(newpkt.bytes(), (self.dst_ip.borrow(), self.dst_port));
                                                if pkt.seqno > seen_seqno { seen_seqno = pkt.seqno; }
                                            },
                                            PacketType::SYNACK => {
                                                // Something we've seen before
                                                if pkt.seqno <= seen_seqno {
                                                    self.socket = Some(socket);
                                                    self.state = ConnState::CONNECTED;
                                                    return Ok(());
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                                None => {}
                            };
                        }
                        Err(_) => {},
                    }
                    seqno += 1;
                    payload = Packet::new(PacketType::SYN, seqno, None);
                    thread::sleep(Duration::from_millis(500));
                }
                Err(ConnectionError::ConnTimeout)
            }
            ConnState::CONNECTED => {
                // Already connected
                Err(ConnectionError::ConnExists)
            }
        }
    }

    pub fn heartbeat(&mut self) -> Result<(), ConnectionError> {
        match self.state {
            ConnState::CONNECTED => {
                match &self.socket {
                    Some(socket) => {
                        let mut pkt = Packet::new(PacketType::HEARTBEAT, 0, None);
                        let _ = socket.send_to(pkt.bytes(), (self.dst_ip.borrow(), self.dst_port));
                    }
                    None => { return Err(ConnectionError::Unknown); }
                }
                Ok(())
            },
            ConnState::DISCON => Err(ConnectionError::ConnDead)
        }
    }
}
