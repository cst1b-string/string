use comm::{peer::PeerError, Peer};
use prisma_orm::prisma::*;
use std::{collections::HashMap, default::Default};

/// Contains the chat_history a user has, their settings
struct UserData {
    /// Maps peer's username to actual struct
    pub peers: HashMap<String, Peer>,
    pub client: PrismaClient,
}

impl UserData {
    pub fn new(client: PrismaClient) -> UserData {
		// get peers from client.
		
		
	}

    pub fn send_message(&self, peer_name: String) -> Result<(), PeerError> {
        match self.peers.get(&peer_name) {
            Some(peer) => peer.send_message(),
            None => PeerError::PeerNotFound,
        }
    }

    pub fn get_dark_mode(&self) -> Option<settings::Data> {
        // self.client.user().find_unique(vec).exec().await.unwrap()
    }
}
