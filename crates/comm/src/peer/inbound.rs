//! This module contains the background task for receiving packets from the network and forwarding
//! their decoded contents to the application.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

use protocol::{try_decode_packet, try_verify_packet_sig, ProtocolPacket, ProtocolPacketType};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, trace};

use crate::{
    maybe_break,
    socket::{SocketPacket, SocketPacketType},
    try_break, try_continue, Peer, Socket,
};

use super::PeerState;

/// Starts the background tasks that handle receiving packets from the network and forwarding their
/// decoded contents to the application.
pub fn start_peer_receiver_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    app_inbound_tx: mpsc::Sender<ProtocolPacket>,
    mut net_inbound_rx: mpsc::Receiver<SocketPacket>,
    remote_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
    packet_number: Arc<Mutex<u32>>,
    packet_acks: Arc<RwLock<HashSet<(u32, u32)>>>,
) {
    tokio::task::spawn(async move {
        // priority queue for packets - this guarantees correct sequencing of UDP
        // packets that make up a single protocol message
        let mut packet_queue = BinaryHeap::new();

        loop {
            trace!("start_peer_receiver_worker loop");

            let packet: SocketPacket = maybe_break!(net_inbound_rx.recv().await);

            // read current state
            let current_state = {
                let state = *state.read().await;
                trace!(state = ?state, "acquire state read lock");
                state
            };
            debug!(
                ?current_state,
                kind = ?packet.packet_type,
                "received packet"
            );

            match current_state {
                PeerState::Init => {
                    match packet.packet_type {
                        SocketPacketType::Syn | SocketPacketType::SynAck => {
                            // initiator never receives SYN or SYNACK
                        }
                        SocketPacketType::Ack => {
                            // write to network
                            try_break!(
                                net_outbound_tx
                                    .send(SocketPacket::empty(
                                        SocketPacketType::SynAck,
                                        packet.packet_number,
                                        0,
                                    ))
                                    .await
                            );
                            // transition to key init state
                            debug!(
                                ?current_state,
                                next = ?PeerState::Established,
                                "state transition"
                            );
                            *state.write().await = PeerState::Established;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Connect => {
                    match packet.packet_type {
                        SocketPacketType::Ack => {
                            // responder never receives ACK
                        }
                        SocketPacketType::Syn => {
                            let ack =
                                SocketPacket::empty(SocketPacketType::Ack, packet.packet_number, 0);
                            // write to network
                            try_break!(
                                net_outbound_tx.send(ack).await,
                                "failed to send packet to network"
                            );
                        }
                        SocketPacketType::SynAck => {
                            debug!(
                                ?current_state,
                                next = ?PeerState::Established,
                                "state transition"
                            );
                            *state.write().await = PeerState::Established;
                        }
                        SocketPacketType::Heartbeat
                        | SocketPacketType::Data
                        | SocketPacketType::Invalid => {}
                    }
                }
                PeerState::Established => match packet.packet_type {
                    SocketPacketType::Syn
                    | SocketPacketType::SynAck
                    | SocketPacketType::Heartbeat
                    | SocketPacketType::Invalid => {}
                    SocketPacketType::Ack => {
                        let mut packets = packet_acks.write().await;
                        packets.remove(&(packet.packet_number, packet.chunk_number));
                    }
                    SocketPacketType::Data => {
                        // send ack
                        try_break!(
                            net_outbound_tx
                                .send(SocketPacket::empty(
                                    SocketPacketType::Ack,
                                    packet.packet_number,
                                    packet.chunk_number,
                                ))
                                .await
                        );

                        // add packet to queue
                        packet_queue.push(Reverse(packet));

                        // attempt to decode
                        let data_len: usize = packet_queue
                            .iter()
                            .map(|Reverse(packet)| packet.compressed_data.len())
                            .sum();

                        let mut buf = Vec::with_capacity(data_len);

                        packet_queue.iter().for_each(|Reverse(packet)| {
                            buf.append(&mut packet.compressed_data.clone())
                        });

                        let packet = match try_decode_packet(buf) {
                            Ok(packet) => packet,
                            Err(_) => continue,
                        };

                        if current_state == PeerState::Established {
                            // clear queue - return early to avoid lots of nesting
                            debug!("clear packet queue");
                            packet_queue.clear();
                            continue;
                        }

                        match packet.packet_type {
                            Some(ProtocolPacketType::PktGossip(ref gossip)) => {
                                // check if we are missing a signed ppacket
                                if let None = gossip.packet {
                                    continue;
                                }

                                let signed_packet = gossip.packet.as_ref().unwrap();

                                let forward = {
                                    let mut peers = peers.write().await;
                                    let peer = match peers.get_mut(&remote_addr) {
                                        Some(p) => p,
                                        None => {
                                            continue;
                                        }
                                    };

                                    // Verify signature on packet
                                    let signed_packet =
                                        try_continue!(try_verify_packet_sig(&signed_packet));

                                    // Dispatch gossip to respective code if its for us...
                                    peer.dispatch_gossip(
                                        signed_packet.clone(),
                                        app_inbound_tx.clone(),
                                    )
                                    .await
                                    .unwrap()
                                };
                                // ..., otherwise, forward it on to our peers
                                if forward {
                                    drop(peers);
                                    let _ =
                                        Socket::forward_gossip(packet, peers.clone(), remote_addr)
                                            .await;
                                }
                            }
                            Some(_) => {}
                            None => {}
                        }

                        // clear queue
                        debug!("clear packet queue");
                        packet_queue.clear();
                    }
                },
                PeerState::Dead => {}
            }
        }
    });
}
