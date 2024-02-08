//! # string-comms
//!
//! This crate contains the communication code for string

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use error::SocketError;
use peer::Peer;
use socket::NetworkPacket;

use protocol::packet::v1::Packet;
use tokio::{
    net::UdpSocket,
    sync::{mpsc, RwLock},
};

mod error;
mod peer;
mod socket;

/// The maximum size of a UDP datagram.
const UDP_MAX_DATAGRAM_SIZE: usize = 65_507;

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

        // start the outbound worker
        start_outbound_worker(socket.clone(), peers.clone());

        Ok(Self {
            inner: socket,
            peers,
        })
    }

    /// Add a new peer to the list of connections, returning a channel for receiving
    /// packets from the peer.
    pub async fn add_peer(
        &mut self,
        addr: SocketAddr,
    ) -> (mpsc::Sender<Packet>, mpsc::Receiver<Packet>) {
        let (peer, app_inbound_rx, net_outbound_rx) = Peer::new(addr, true);
        let app_outbound_tx = peer.app_outbound_tx.clone();

        // spawn the inbound peer task
        spawn_inbound_peer_task(self.inner.clone(), peer.destination, net_outbound_rx);

        // insert the peer into the connections map - done in a separate block to avoid holding the
        // lock for too long
        {
            let mut connections = self.peers.write().await;
            connections.insert(addr, peer);
        }

        (app_outbound_tx, app_inbound_rx)
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

/// Start the outbound network worker.
fn start_outbound_worker(socket: Arc<UdpSocket>, peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>) {
    tokio::spawn(async move {
        let mut buf = [0; UDP_MAX_DATAGRAM_SIZE];
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
            if let Err(e) = peer.net_inbound_tx.send(packet).await {
                eprintln!("Error forwarding packet to peer: {:?}", e);
            }
        }
    });
}

/// Starts the background tasks that handle receiving
fn spawn_inbound_peer_task(
    socket: Arc<UdpSocket>,
    destination: SocketAddr,
    mut net_outbound_rx: mpsc::Receiver<NetworkPacket>,
) {
    tokio::spawn(async move {
        loop {
            // receive packet from peer
            let packet = match net_outbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            // encode packet
            let bytes = match packet.encode() {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error encoding packet: {:?}", e);
                    continue;
                }
            };

            // send to network
            if let Err(e) = socket.send_to(&bytes, destination).await {
                eprintln!("Error sending packet to network: {:?}", e);
            }
        }
    });
}
