use std::{collections::HashSet, sync::Arc, time::Duration};

use tokio::{
    select,
    sync::{mpsc, RwLock},
    time::sleep,
};
use tracing::debug;

use crate::{socket::SocketPacket, try_break};

use super::PeerState;

/// Periodically checks if we've received an ACK for a packet, and if not, resends the packet.
/// Times out after 30s and transitions the peer to the dead state.
pub fn start_ack_timeout_worker(
    state: Arc<RwLock<PeerState>>,
    packet_acks: Arc<RwLock<HashSet<(u32, u32)>>>,
    net_outbound_tx: mpsc::Sender<SocketPacket>,
    net_packet: SocketPacket,
) {
    // spawn a new task that keeps checking if we've received an ACK yet
    // if we haven't, resend the packet
    tokio::spawn(async move {
        let timeout = Duration::from_secs(30);
        let (packet_number, chunk_number) = (net_packet.packet_number, net_packet.chunk_number);

        select! {
            _ = sleep(timeout) => {
                debug!("packet with number {} chunk {} did not receive an ACK in 30s - peer dead", packet_number, chunk_number);
                *state.write().await = PeerState::Dead;
            },

            _ = async {
                loop {
                    // wait for 1s before checking if we've received an ACK
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    let has_packet = { packet_acks.read().await.contains(&(packet_number, chunk_number))};
                    if !has_packet {
                        break;
                    }
                    // retransmit
                    try_break!(net_outbound_tx.send(net_packet.clone()).await);
                }
            } => {}
        }
    });
}
