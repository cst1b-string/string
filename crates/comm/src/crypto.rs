//! Handles the Double-Ratchet (DR) key exchange for communications

use thiserror::Error;
use protocol::{ProtocolPacket, packet, crypto};
use crate::peer::PeerState;
use double_ratchet_rs::Ratchet;
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};
use std::fmt;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Received non crypto packet before key exchange")]
    NonCryptoFail,
}

pub struct Crypto {
    ratchet: Option<Ratchet>,
    shared_secret: Option<SharedSecret>,
    dh_secret: EphemeralSecret
}

impl fmt::Debug for Crypto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Opaque Crypto object>")
    }
}

impl Crypto {
	pub fn new() -> Self {
		let dh = EphemeralSecret::random_from_rng(OsRng);
		Self {
			ratchet: None,
			shared_secret: None,
			dh_secret: dh
		}
	}

	pub fn handle_crypto(
		&mut self,
		packet: ProtocolPacket, 
		state: PeerState
	) -> Result<(), CryptoError> {
		match packet.packet {
			Some(packet::v1::packet::Packet::PktMessage(_)) => {
				return Err(CryptoError::NonCryptoFail);
			}
	        Some(packet::v1::packet::Packet::PktCrypto(_crypto_pkt)) => {
	        	return Ok(());
	        }
	        None => { return Err(CryptoError::NonCryptoFail); }
		}
	}

	pub fn kex_packet(&self) -> ProtocolPacket {
		let mut pkt = ProtocolPacket::default();
		let crypto = crypto::v1::Crypto {
			dh_key: vec![],
			dr_key: vec![],
	    };
	    pkt.packet = Some(packet::v1::packet::Packet::PktCrypto(crypto));
	    pkt
	}
}