use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use rand::{rngs::OsRng, seq::IteratorRandom};
use string_protocol::{MessageType, ProtocolPacket};
use tokio::sync::{mpsc, RwLock};
use tracing::trace;

use crate::Peer;

/// Number of peers to send gossip to
const GOSSIP_COUNT: usize = 3;

/// Enumeration of gossip action types.
pub enum GossipAction {
    /// Send a normal unencrypted packet to some peers via gossip
    Send,
    /// Same as above, but encrypted. This should be the common case
    SendEncrypted,
    /// We received a gossip packet, please forward it
    Forward,
    /// Actually not a gossip, just send directly
    SendDirect,
}

pub struct Gossip {
    /// What to do with the gossip
    pub action: GossipAction,
    /// When this gossip is received from, is None if sending from current node
    pub addr: Option<SocketAddr>,
    /// Gossip packet to forward (either this or the one below)
    pub packet: Option<ProtocolPacket>,
    /// Gossip message to forward (either this or the one above)
    pub message: Option<MessageType>,
    /// Destination to send to; not needed when forwarding
    pub dest: Option<String>,
    ///
    pub dest_sockaddr: Option<SocketAddr>,
}

pub fn start_gossip_worker(
    mut gossip_rx: mpsc::Receiver<Gossip>,
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,
) {
    tokio::spawn(async move {
        loop {
            trace!("start gossip task loop");

            // receive gossip
            let Gossip {
                action,
                addr: skip,
                packet,
                message,
                dest,
                dest_sockaddr,
            } = match gossip_rx.recv().await {
                Some(gossip) => gossip,
                None => break,
            };

            if let GossipAction::SendDirect = action {
                let mut peers_obj = peers.write().await;
                let peer = peers_obj.get_mut(&dest_sockaddr.unwrap());
                if peer.is_none() {
                    continue;
                }
                let peer_ = peer.unwrap();
                let peername = peer_.peername.clone();
                let _ = peer_
                    .send_gossip_single(message.unwrap().clone(), peername.unwrap())
                    .await;
                continue;
            }

            // Selects at most 3 peers randomly from list of peers - should
            // probably employ round robin here.
            let targets: Vec<_> = peers
                .read()
                .await
                .keys()
                // skip if included
                .filter(|addr| skip.map(|skip_addr| skip_addr != **addr).unwrap_or(true))
                .cloned()
                .choose_multiple(&mut OsRng, GOSSIP_COUNT);

            // we have no targets!
            if targets.is_empty() {
                continue;
            }

            for target in targets {
                let mut peers_write = peers.write().await;
                let target_peer = peers_write.get_mut(&target);
                let target_peer_ = target_peer.expect("No such peer");
                let res = match action {
                    GossipAction::Send => {
                        trace!("sending gossip {:?}", message);
                        target_peer_
                            .send_gossip_single(message.clone().unwrap(), dest.clone().unwrap())
                            .await
                    }
                    GossipAction::SendEncrypted => {
                        trace!("sending encrypted gossip {:?}", packet);
                        target_peer_
                            .send_gossip_single_encrypted(
                                packet.clone().unwrap(),
                                dest.clone().unwrap(),
                            )
                            .await
                    }
                    GossipAction::Forward => {
                        trace!("forwarding gossip {:?}", packet);
                        target_peer_.send_packet(packet.clone().unwrap()).await
                    }
                    GossipAction::SendDirect => unreachable!(),
                };
                if res.is_ok() {}
            }
        }
    });
}
