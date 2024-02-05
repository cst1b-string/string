//! # string-comms
//!
//! This crate contains the communication code for string

use std::{net::SocketAddr, sync::Arc};

use error::SocketError;
use packet::{NetworkPacket, NetworkPacketType};
use peer::Peer;
use protocol::packet::v1::Packet;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex, RwLock},
};

mod error;
mod packet;
mod peer;

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    inner: Arc<UdpSocket>,
    /// A list of connections to other peers.
    peers: Arc<RwLock<Vec<Peer>>>,
    /// The send half of the packet transmission channel.
    packet_tx: mpsc::Sender<Packet>,
    /// The receive half of the packet transmission channel.
    packet_rx: Arc<Mutex<mpsc::Receiver<Packet>>>,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address.
    pub async fn bind(addr: SocketAddr) -> Result<Self, SocketError> {
        let socket = UdpSocket::bind(addr).await.map_err(SocketError::IoError)?;
        let (packet_tx, packet_rx) = mpsc::channel(32);

        Ok(Self {
            inner: Arc::new(socket),
            packet_tx,
            packet_rx: Arc::new(Mutex::new(packet_rx)),
            peers: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Returns a clone of the sender half of the packet transmission channel. This
    /// is used to send packets to the network.
    pub fn sender(&self) -> mpsc::Sender<Packet> {
        self.packet_tx.clone()
    }

    /// Add a new peer to the list of connections.
    pub async fn add_peer(&mut self, addr: SocketAddr) {
        let mut connections = self.peers.write().await;
        connections.push(addr.into());
    }

    /// Spawns a new task to handle heartbeat events.
    pub async fn spawn_network_task(&self) {
        let socket = self.inner.clone();
        let peers = self.peers.clone();
        let packet_rx = self.packet_rx.clone();

        // outbound packet task
        tokio::spawn(async move {
            loop {
                let packet = match packet_rx.lock().await.recv().await {
                    Some(packet) => packet,
                    None => break,
                };
                todo!("send packet to peer")
            }
        });

        // inbound packet task
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            loop {
                // wait for socket to be readable
                if let Err(e) = socket.readable().await {
                    eprintln!("Error reading from socket: {:?}", e);
                    break;
                }

                // iterate over all peers and read packets
                let (size, addr) = match socket.recv_from(&mut buf).await {
                    Ok((size, addr)) => (size, addr),
                    Err(e) => {
                        eprintln!("Error reading from network: {:?}", e);
                        continue;
                    }
                };
            }
        });
    }

    /// Send a packet to the given peer.
    pub async fn send_packet(
        &self,
        packet: &Packet,
        peer_addr: SocketAddr,
    ) -> Result<(), SocketError> {
        // encode protocol message, then wrap in frame
        let buf = protocol::try_encode_packet(&packet).map_err(|e| SocketError::EncodeFail(e))?;
        let buf = NetworkPacket::new(NetworkPacketType::Data, 0, &buf).encode()?;

        // send datagram
        self.inner
            .send_to(&buf, peer_addr)
            .await
            .map_err(SocketError::IoError)?;

        Ok(())
    }
}
