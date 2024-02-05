//! # string-comms
//!
//! This crate contains the communication code for string

use std::{net::SocketAddr, sync::Arc};

use connection::{Peer, PeerState};
use error::SocketError;
use packet::{NetworkPacket, NetworkPacketType};
use protocol::packet::v1::Packet;
use tokio::{net::UdpSocket, sync::RwLock};

mod connection;
mod error;
mod packet;

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    inner: Arc<UdpSocket>,
    /// A list of connections to other peers.
    peers: Arc<RwLock<Vec<Peer>>>,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address.
    pub async fn bind(addr: SocketAddr) -> Result<Self, SocketError> {
        // bind to the target address
        let socket = UdpSocket::bind(addr).await.map_err(SocketError::IoError)?;
        Ok(Self {
            inner: Arc::new(socket),
            peers: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Add a new peer to the list of connections.
    pub async fn add_peer(&mut self, addr: SocketAddr) {
        let mut connections = self.peers.write().await;
        connections.push(addr.into());
    }

    /// Spawns a new task to handle heartbeat events.
    pub async fn spawn_network_task(&self) {
        let peers = self.peers.clone();

        tokio::spawn(async move {
            loop {
                for peer in peers.read().await.iter() {
                    match peer.state {
                        PeerState::Disconnected => todo!(),
                        PeerState::Connecting => todo!(),
                        PeerState::Connected => todo!(),
                    }
                }
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
