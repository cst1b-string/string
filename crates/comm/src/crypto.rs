//! Handles the Double-Ratchet (DR) key exchange for communications

use crate::peer::PeerState;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use double_ratchet_rs::{Header, Ratchet};
use protocol::{crypto, packet::v1::packet::PacketType, ProtocolPacket};
use rand::rngs::OsRng;
use std::{fmt, io::Cursor, mem};
use thiserror::Error;
use tracing::debug;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret, StaticSecret};

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Received non crypto packet before key exchange")]
    NonKexFail,
    // Ratchet used before it is initialised
    // We should not have this error
    #[error("Ratchet not initialised")]
    MissingRatchet,
    // Currently generic, todo make more specific
    #[error("Ciphertext is bad")]
    BadCiphertext,
}

/// An enum to handle the Double-Ratchet (DR) key exchange for communications.
pub enum Crypto {
    /// An initiator of a key exchange.
    Initiator {
        dh_pubkey: StaticSecret,
        dr_pubkey: EphemeralSecret,
    },
    /// A responder to a key exchange.
    Responder { dh_pubkey: StaticSecret },
    /// An initialized Crypto object, with associated data.
    Initialized {
        ratchet: Ratchet,
        associated_data: Vec<u8>,
    },
}

impl fmt::Debug for Crypto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Opaque Crypto object>")
    }
}

impl Crypto {
    /// Create a new [Crypto] instance as an initiator.
    pub fn new_initiator() -> Self {
        Self::Initiator {
            dh_pubkey: StaticSecret::random_from_rng(OsRng),
            dr_pubkey: EphemeralSecret::random_from_rng(OsRng),
        }
    }

    /// Create a new [Crypto] instance as a responder.
    pub fn new_responder() -> Self {
        Self::Responder {
            dh_pubkey: StaticSecret::random_from_rng(OsRng),
        }
    }

    /// Handle a key exchange packet, updating the internal state of the [Crypto] instance.
    pub fn handle_kex(
        &mut self,
        packet: ProtocolPacket,
        // Currently state not needed, but just in case
        _state: PeerState,
    ) -> Result<(), CryptoError> {
        // ensure we are not already initialized
        let (dh_pubkey, dr_pubkey) = match self {
            Crypto::Initiator {
                dh_pubkey,
                dr_pubkey,
            } => (dh_pubkey, Some(dr_pubkey)),
            Crypto::Responder { dh_pubkey } => (dh_pubkey, None),
            Crypto::Initialized { .. } => return Err(CryptoError::NonKexFail),
        };

        match packet.packet_type {
            // all other packet types are invalid
            Some(PacketType::PktMessage(_))
            | Some(PacketType::PktFirst(_))
            | Some(PacketType::PktGossip(_))
            | None => Err(CryptoError::NonKexFail),
            // if we receive a crypto packet, we can proceed
            Some(PacketType::PktCrypto(packet)) => {
                let peer_dh_pubkey_bytes: [u8; 32] = packet.dh_pubkey[..32].try_into().unwrap();
                let peer_dh_pubkey = PublicKey::from(peer_dh_pubkey_bytes);
                let shared_secret = dh_pubkey.diffie_hellman(&peer_dh_pubkey);

                debug!(
                    shared_secret = hex::encode(shared_secret.as_bytes()),
                    "shared secret established"
                );

                if packet.dr_pubkey.is_empty() {
                    let (ratchet, dr_pubkey) = Ratchet::init_bob(shared_secret.to_bytes());
                    debug!(
                        pubkey = hex::encode(dr_pubkey.clone().as_bytes()),
                        "init_bob and generated"
                    );

                    *self = Crypto::Initialized {
                        ratchet,
                        associated_data: vec![],
                    }
                } else {
                    let peer_dr_pubkey_bytes: [u8; 32] = packet.dr_pubkey[..32].try_into().unwrap();
                    let peer_dr_pubkey = PublicKey::from(peer_dr_pubkey_bytes);
                    debug!(
                        pubkey = hex::encode(peer_dr_pubkey.clone().as_bytes()),
                        "init_alice pubkey"
                    );
                    let ratchet = Ratchet::init_alice(shared_secret.to_bytes(), peer_dr_pubkey);

                    *self = Crypto::Initialized {
                        ratchet,
                        associated_data: vec![],
                    }
                }

                Ok(())
            }
        }
    }

    /// Create a new key exchange packet.
    pub fn generate_kex_packet(&self) -> ProtocolPacket {
        // if the ratchet is not initialised, we cannot decrypt
        let (dh_pubkey, dr_pubkey) = match self {
            Crypto::Initiator {
                dh_pubkey,
                dr_pubkey,
            } => (dh_pubkey, Some(dr_pubkey)),
            Crypto::Responder { dh_pubkey } => (dh_pubkey, None),
            Crypto::Initialized { .. } => {
                panic!("Ratchet should not be initialised at this point");
            }
        };

        // get public keys
        let dh_pubkey = PublicKey::from(dh_pubkey).as_bytes().to_vec();
        let dr_pubkey = dr_pubkey
            .map(|pk| PublicKey::from(pk).as_bytes().to_vec())
            .unwrap_or_default();

        // create a new packet
        let mut packet = ProtocolPacket::default();
        let contents = crypto::v1::Crypto {
            dh_pubkey,
            dr_pubkey,
        };
        packet.packet_type = Some(PacketType::PktCrypto(contents));
        packet
    }

    /// Encrypt the data using the ratchet, advancing the ratchet state in the process.
    pub fn encrypt(&mut self, data: &Vec<u8>) -> Result<Vec<u8>, CryptoError> {
        // if the ratchet is not initialised, we cannot decrypt
        let (ratchet, associated_data) = match self {
            Crypto::Initiator { .. } | Crypto::Responder { .. } => {
                return Err(CryptoError::MissingRatchet)
            }
            Crypto::Initialized {
                ratchet,
                associated_data,
                ..
            } => (ratchet, associated_data),
        };
        let (header, encrypted, nonce) = ratchet.encrypt(data, associated_data);
        debug!(
            header = hex::encode(Vec::<u8>::from(header.clone())),
            encrypted = hex::encode(encrypted.clone()),
            nonce = hex::encode(nonce),
            "encrypted ended"
        );

        let mut size_encoded = Vec::new();
        let _ = size_encoded.write_u64::<BigEndian>(encrypted.len().try_into().unwrap());
        let ciphertext = [size_encoded, Vec::from(header), encrypted, nonce.to_vec()].concat();

        Ok(ciphertext)
    }

    /// Decrypt the data using the ratchet, advancing the ratchet state in the process.
    pub fn decrypt(&mut self, data: &Vec<u8>) -> Result<Vec<u8>, CryptoError> {
        // if the ratchet is not initialised, we cannot decrypt
        let (ratchet, associated_data) = match self {
            Crypto::Initiator { .. } | Crypto::Responder { .. } => {
                return Err(CryptoError::MissingRatchet)
            }
            Crypto::Initialized {
                ratchet,
                associated_data,
                ..
            } => (ratchet, associated_data),
        };

        let mut cursor = Cursor::new(data);
        let size: usize = match cursor.read_u64::<BigEndian>() {
            Ok(s) => s.try_into().unwrap(),
            Err(_) => {
                return Err(CryptoError::BadCiphertext);
            }
        };

        // First 8 bytes is u64 size
        let ciphertext = data[8..].to_vec();
        let header_start: usize = ciphertext.len() - size - 12;
        let nonce_start: usize = header_start + size;

        let header = Header::from(&ciphertext[..header_start]);
        let encrypted = &ciphertext[header_start..nonce_start];
        let nonce: [u8; 12] = ciphertext[nonce_start..].try_into().unwrap();
        debug!(
            header = hex::encode(Vec::<u8>::from(header.clone())),
            encrypted = hex::encode(encrypted),
            nonce = hex::encode(nonce),
            "decryption started"
        );

        let decrypted = ratchet.decrypt(&header, encrypted, &nonce, associated_data);
        Ok(decrypted)
    }
}
