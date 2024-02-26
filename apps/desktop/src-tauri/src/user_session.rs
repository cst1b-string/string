use comm::{peer::PeerError, Peer};
use prisma_orm::prisma::*;
use std::{collections::HashMap, default::Default};

/// Contains the chat_history a user has, their settings
struct UserSession {
    /// Maps peer's username to actual struct
    pub peers: HashMap<String, Peer>,
    pub client: PrismaClient,
}

impl UserSession {
    pub fn send_message(&self, peer_name: String) -> Result<(), PeerError> {
        match self.peers.get(&peer_name) {
            Some(peer) => peer.send_message(),
            None => PeerError::PeerNotFound,
        }
    }

    pub fn get_dark_mode(&self) -> Option<settings::Data> {
        self.client.settings().find_unique(is_dark_mode)
    }
}
