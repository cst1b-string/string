//! # string-comms
//!
//! This crate contains the communication code for string

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use error::SocketError;
use packet::{NetworkPacket, NetworkPacketType};
use peer::Peer;
use protocol::packet::v1::Packet;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};

mod error;
mod packet;
mod peer;

/// A wrapper around the [UdpSocket] type that provides a higher-level interface for sending and
/// receiving packets from multiple peers.
pub struct Socket {
    /// The inner [UdpSocket] used for sending and receiving packets.
    inner: Arc<UdpSocket>,
    /// A map of connections to other peers.
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
    /// The send half of the packet transmission channel.
    packet_tx: mpsc::Sender<NetworkPacket>,
}

impl Socket {
    /// Create a new `Socket` that is bound to the given address. This method also
    /// starts the background tasks that handle sending and receiving packets.
    pub async fn bind(addr: SocketAddr) -> Result<Self, SocketError> {
        // bind socket
        let socket: Arc<_> = UdpSocket::bind(addr)
            .await
            .map_err(SocketError::IoError)?
            .into();

        // create peers map
        let peers = Arc::new(RwLock::new(HashMap::new()));

        // create mpsc channel for packet transmission
        let (packet_tx, mut packet_rx) = mpsc::channel(32);

        // outbound packet task - uses `packet_rx` to process outbound packets
        let outbound_peers = peers.clone();
        let outbound_socket = socket.clone();
        tokio::spawn(async move {
            let socket = outbound_socket;
            let peers = outbound_peers;

            loop {
                let packet: NetworkPacket = match packet_rx.recv().await {
                    Some(packet) => packet,
                    None => break,
                };

                // lookup peer
                let peer = match peers.read().await.get(&packet.addr) {
                    Some(peer) => peer,
                    None => {
                        eprintln!("Unknown peer: {:?}", packet.addr);
                        continue;
                    }
                };

                peer.inbound_packet_tx.send(packet).await;
            }
        });

        // inbound packet task
        let inbound_socket = socket.clone();
        let inbound_packet_tx = packet_tx.clone();
        tokio::spawn(async move {
            let socket = inbound_socket;
            let packet_tx = inbound_packet_tx;
            let mut buf = [0; 1024];
            loop {
                // wait for socket to be readable
                if let Err(e) = socket.readable().await {
                    eprintln!("Error reading from socket: {:?}", e);
                    break;
                }

                // receive packet
                let (size, addr) = match socket.recv_from(&mut buf).await {
                    Ok((size, addr)) => (size, addr),
                    Err(e) => {
                        eprintln!("Error reading from network: {:?}", e);
                        continue;
                    }
                };

                // see if we know this peer
                let mut peers = peers.write().await;
                let peer = match peers.get_mut(&addr) {
                    Some(peer) => peer,
                    None => {
                        eprintln!("Unknown peer: {:?}", addr);
                        continue;
                    }
                };

                // decode network packet
                let packet = match NetworkPacket::decode(&buf[..size]) {
                    Ok(packet) => packet,
                    Err(e) => {
                        eprintln!("Error decoding packet: {:?}", e);
                        continue;
                    }
                };

                // forward to peer
                peer.packet_tx.send(packet).await;
            }
        });

        Ok(Self {
            inner: socket,
            packet_tx,
            peers,
        })
    }

    /// Add a new peer to the list of connections.
    pub async fn add_peer(&mut self, addr: SocketAddr) {
        let mut connections = self.peers.write().await;
        let peer = Peer::new(self.inner.clone(), addr);
        connections.insert(addr, peer);
    }

    /// Send a packet to the given peer.
    pub async fn send_packet(
        &self,
        packet: Packet,
        peer_addr: SocketAddr,
    ) -> Result<(), SocketError> {
        // encode protocol message, then wrap in frame
        let buf = protocol::try_encode_packet(&packet).map_err(|e| SocketError::EncodeFail(e))?;
        let packet = NetworkPacket::new(NetworkPacketType::Data, 0, &buf);

        // forward to network thread
        self.packet_tx.send(packet).await?;

        Ok(())
    }
}
