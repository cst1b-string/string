//! # string-comms
//!
//! This crate contains the communication code for string

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use error::SocketError;
use packet::NetworkPacket;
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
        tokio::spawn(async move {
            let peers = outbound_peers;

            loop {
                let (addr, packet) = match packet_rx.recv().await {
                    Some(p) => p,
                    None => break,
                };
                // lookup peer
                let peers = peers.read().await;
                let peer: &Peer = match peers.get(&addr) {
                    Some(peer) => peer,
                    None => {
                        eprintln!("Unknown peer: {:?}", addr);
                        continue;
                    }
                };

                peer.app_outbound_tx.send(packet).await;
            }
        });

        // inbound packet task
        let inbound_socket = socket.clone();
        let inbound_peers = peers.clone();
        tokio::spawn(async move {
            let socket = inbound_socket;
            let peers = inbound_peers;
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
                peer.net_inbound_tx.send(packet).await;
            }
        });

        Ok(Self {
            inner: socket,
            peers,
        })
    }

    /// Add a new peer to the list of connections.
    pub async fn add_peer(&mut self, addr: SocketAddr) {
        let mut connections = self.peers.write().await;
        let (peer, _, _) = Peer::new(addr, false);
        connections.insert(addr, peer);
    }

    /// Send a packet to the given peer.
    pub async fn send_packet(
        &mut self,
        packet: Packet,
        peer_addr: SocketAddr,
    ) -> Result<(), SocketError> {
        // lookup the peer
        let mut peers = self.peers.write().await;
        let peer = peers.get_mut(&peer_addr).ok_or(SocketError::Unknown)?;
        peer.send_packet(packet).await?;

        Ok(())
    }
}
