use std::{sync::Arc, time::Duration};

use protocol::{try_encode_packet, ProtocolPacket};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, trace, warn};

use crate::{
    maybe_break,
    peer::MAX_PROTOCOL_PACKET_CHUNK_SIZE,
    socket::{SocketPacket, SocketPacketType},
    try_break, try_continue,
};

use super::PeerState;

/// Starts the background task that handles sending packets to the network, taking
/// packets from the application, encoding them as [SocketPacket]s, before sending them to the network.
pub fn start_peer_sender_worker(
    state: Arc<RwLock<PeerState>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    mut app_outbound_rx: mpsc::Receiver<ProtocolPacket>,
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
                        .send(SocketPacket::new(
                            SocketPacketType::Syn,
                            syns_sent,
                            0,
                            vec![],
                        ))
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

            // encode packet
            trace!("encode packet: {:?}", packet);
            let buf = try_continue!(try_encode_packet(&packet), "Failed to encode packet");

            // split packet into network packets and send
            for net_packet in buf
                .chunks(MAX_PROTOCOL_PACKET_CHUNK_SIZE)
                .map(|chunk| SocketPacket::new(SocketPacketType::Data, 0, 0, chunk))
            {
                try_break!(net_outbound_tx.send(net_packet).await);
            }
        }
    });
}
