//! This module contains the background task for sending packets to the network, taking packets from
//! the application, encoding them as [SocketPacket]s, then sending them to the network.

use std::{collections::HashSet, sync::Arc, time::Duration};

use string_protocol::{try_encode_packet, ProtocolPacket};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, trace, warn};

use crate::{
    crypto::Crypto,
    maybe_break,
    peer::{ack::start_ack_timeout_worker, MAX_PROTOCOL_PACKET_CHUNK_SIZE},
    socket::{SocketPacket, SocketPacketType},
    try_break, try_continue,
};

use super::PeerState;

/// Starts the background task that handles sending packets to the network, taking
/// packets from the application, encoding them as [NetworkPacket]s, before sending them to the network.
pub fn start_peer_sender_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    mut app_outbound_rx: mpsc::Receiver<ProtocolPacket>,
    _crypto: Arc<RwLock<Crypto>>,
    packet_number: Arc<Mutex<u32>>,
    pending_acks: Arc<RwLock<HashSet<(u32, u32)>>>,
) {
    tokio::task::spawn(async move {
        let mut syns_sent: u32 = 0;
        loop {
            trace!("start_peer_sender_worker loop");
            // ensure we're in a state where we can send packets
            let current_state = { *state.read().await };
            if current_state == PeerState::Dead {
                warn!("peer is dead, breaking out of sender worker loop");
                break;
            }
            // Send syn regardless of which end we are
            // Only the receiving side will acknowledge
            if current_state == PeerState::Init || current_state == PeerState::Connect {
                try_break!(
                    net_outbound_tx
                        .send(
                            SocketPacket::new(SocketPacketType::Syn, syns_sent, 0, vec![])
                                .expect("failed to create packet")
                        )
                        .await
                );
                syns_sent += 1;
            }

            // if we're not established, go around again
            if current_state != PeerState::Established {
                debug!("peer is not established, sleeping for 500ms");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            // receive packet from queue
            trace!("receive packet from queue");
            let packet: ProtocolPacket = maybe_break!(app_outbound_rx.recv().await);

            // these locks may cause some contention - investigate
            let mut packet_number = packet_number.lock().await;
            let mut pending_acks_write = pending_acks.write().await;

            let packets =
                if let Some(ProtocolPacket::PktRequestAvailablePeers(_)) = packet.packet_type {
                    vec![SocketPacket::empty(
                        SocketPacketType::RequestAvailablePeers,
                        *packet_number,
                        0,
                    )]
                    .into_iter()
                } else {
                    // encode packet
                    trace!("encode packet: {:?}", packet);
                    let buf = try_continue!(try_encode_packet(&packet), "Failed to encode packet");

                    buf.chunks(MAX_PROTOCOL_PACKET_CHUNK_SIZE).enumerate().map(
                        |(chunk_idx, chunk)| {
                            SocketPacket::new(
                                SocketPacketType::Data,
                                *packet_number,
                                chunk_idx as u32,
                                chunk,
                            )
                            .expect("failed to create data packet")
                        },
                    );
                };
            // split packet into network packets and send
            for net_packet in packets {
                trace!("sending packet chunk: {:?}", net_packet);
				// TODO: match for packet_type and if RequestAvailablePeers
				// wait for SendAvailablePeers and SEND back an Ack

                match net_outbound_tx.send(net_packet.clone()).await {
                    Ok(_) => {
                        // add the packet to hashmap of packets that we don't have a ACK to
                        pending_acks_write
                            .insert((net_packet.packet_number, net_packet.chunk_number));

                        // start a task that will wait for an ACK for this packet
                        start_ack_timeout_worker(
                            state.clone(),
                            pending_acks.clone(),
                            net_outbound_tx.clone(),
                            net_packet,
                        );
                    }
                    Err(_) => break,
                };
            }
            *packet_number += 1;
        }
    });
}
