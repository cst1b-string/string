use std::net::SocketAddr;

use protocol::ProtocolPacket;
use tokio::sync::mpsc;

use crate::crypto::DoubleRatchet;

/// A node in the network.
pub struct Node {
    /// The node's identity.
    identity: String,
    /// The node's double ratchet.
    double_ratchet: DoubleRatchet,
    /// The channel to the node.
    channel: Channel,
}

impl Node {
    /// Create a new peer node.
    pub fn new_peer(identity: String, addr: SocketAddr, is_initiator: bool) -> Self {
        // the channels for forwarding messages to the peer
        let (direct_tx, direct_rx) = mpsc::channel(32);

        Self {
            identity,
            double_ratchet: DoubleRatchet::new(is_initiator),
            channel: Channel::Peer { addr, direct_tx },
        }
    }

    /// Create a new indirect node.
    pub fn new_indirect(
        identity: String,
        gossip_tx: mpsc::Sender<ProtocolPacket>,
        is_initiator: bool,
    ) -> Self {
        Self {
            identity,
            double_ratchet: DoubleRatchet::new(is_initiator),
            channel: Channel::Indirect { gossip_tx },
        }
    }
}

/// A channel via which we can communicate with a node. There are two types of channels:
/// - A direct channel to a peer node.
/// - An indirect channel to a node via gossip.
pub enum Channel {
    /// A peer node to which we can send messages directly.
    Peer {
        /// The address of the peer.
        addr: SocketAddr,
        /// The direct channel of this [Peer].
        direct_tx: mpsc::Sender<ProtocolPacket>,
    },
    /// An indirect node to which we can send messages via gossip.
    Indirect {
        /// The gossip channel of the [Socket].
        gossip_tx: mpsc::Sender<ProtocolPacket>,
    },
}
