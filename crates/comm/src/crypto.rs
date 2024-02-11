//! Handles the Double-Ratchet (DR) key exchange for communications

use thiserror::Error;
use protocol::{ProtocolPacket, packet, crypto};
use crate::peer::PeerState;
use double_ratchet_rs::{Ratchet, Header};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};
use std::{fmt, mem, io::Cursor};
use tracing::{debug};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

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

pub struct Crypto {
    ratchet: Option<Ratchet>,
    shared_secret: Option<SharedSecret>,
    dh_secret: EphemeralSecret,
    dh_pubkey: PublicKey,
    dr_pubkey: Option<PublicKey>,
    associated_data: Vec<u8>,
}

impl fmt::Debug for Crypto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Opaque Crypto object>")
    }
}

impl Crypto {
    pub fn new() -> Self {
        let dh_secret = EphemeralSecret::random_from_rng(OsRng);
        let dh_pubkey = PublicKey::from(&dh_secret);
        Self {
            ratchet: None,
            shared_secret: None,
            dh_secret: dh_secret,
            dh_pubkey: dh_pubkey,
            dr_pubkey: None,
            associated_data: "Associated Data".into(),
        }
    }

    pub fn handle_kex(
        &mut self,
        packet: ProtocolPacket, 
        // Currently state not needed, but just in case
        _state: PeerState
    ) -> Result<(), CryptoError> {
        match packet.packet {
            Some(packet::v1::packet::Packet::PktMessage(_))
            | Some(packet::v1::packet::Packet::PktFirst(_)) => {
                return Err(CryptoError::NonKexFail);
            }
            Some(packet::v1::packet::Packet::PktCrypto(crypto_pkt)) => {
                let peer_dh_pubkey_bytes: [u8; 32] = crypto_pkt.dh_pubkey[..32].try_into().unwrap();
                let peer_dh_pubkey = PublicKey::from(peer_dh_pubkey_bytes);

                // EphemeralSecret is taken ownership and destroyed by diffie_hellman
                // So we just put in a fake new key
                let fake_key = EphemeralSecret::random_from_rng(OsRng);

                let shared_ = mem::replace(&mut self.dh_secret, fake_key).diffie_hellman(&peer_dh_pubkey);
                self.shared_secret = Some(shared_);

                match &self.shared_secret {
                    Some(shared) => {
                        debug!(shared_secret = hex::encode(shared.as_bytes()), "shared secret established");
                        if crypto_pkt.dr_pubkey.len() == 0 {
                            let (ratchet, dr_pubkey) = Ratchet::init_bob(shared.to_bytes());
                            debug!(pubkey = hex::encode(dr_pubkey.clone().as_bytes()),
                                  "init_bob and generated");
                            self.dr_pubkey = Some(dr_pubkey);
                            self.ratchet = Some(ratchet);
                        }
                        else {
                            let peer_dr_pubkey_bytes: [u8; 32] = crypto_pkt.dr_pubkey[..32].try_into().unwrap();
                            let peer_dr_pubkey = PublicKey::from(peer_dr_pubkey_bytes);
                            debug!(pubkey = hex::encode(peer_dr_pubkey.clone().as_bytes()),
                                  "init_alice pubkey");
                            self.ratchet = Some(Ratchet::init_alice(shared.to_bytes(), peer_dr_pubkey));
                        }
                    }
                    None => {}
                }

                return Ok(());
            }
            None => { return Err(CryptoError::NonKexFail); }
        }
    }

    pub fn kex_packet(&self) -> ProtocolPacket {
        let mut pkt = ProtocolPacket::default();
        let dr_pubkey = match self.dr_pubkey {
            Some(p) => p.as_bytes().to_vec(),
            None => vec![]
        };
        let crypto = crypto::v1::Crypto {
            dh_pubkey: self.dh_pubkey.as_bytes().to_vec(),
            dr_pubkey: dr_pubkey,
        };
        pkt.packet = Some(packet::v1::packet::Packet::PktCrypto(crypto));
        pkt
    }

    pub fn encrypt(&mut self, data: &Vec<u8>) -> Result<Vec<u8>, CryptoError> {
        match &mut self.ratchet {
            Some(ratchet) => {
                let (header, encrypted, nonce) = ratchet.encrypt(data, &self.associated_data);
                debug!(header=hex::encode(Vec::<u8>::from(header.clone())),
                       encrypted=hex::encode(encrypted.clone()),
                       nonce=hex::encode(nonce),
                       "encrypted ended");
                let mut size_encoded = Vec::new();
                let _ = size_encoded.write_u64::<BigEndian>(encrypted.len().try_into().unwrap());
                let ciphertext = [
                    size_encoded,
                    Vec::from(header),
                    encrypted,
                    nonce.to_vec()
                ].concat();
                return Ok(ciphertext);
            }
            None => { return Err(CryptoError::MissingRatchet); }
        }
    }

    pub fn decrypt(&mut self, data: &Vec<u8>) -> Result<Vec<u8>, CryptoError> {
        match &mut self.ratchet {
            Some(ratchet) => {
                // nonce is 12 bytes
                let mut cursor = Cursor::new(data);
                let size: usize = match cursor.read_u64::<BigEndian>() {
                    Ok(s) => s.try_into().unwrap(),
                    Err(_) => { return Err(CryptoError::BadCiphertext); }
                };
                // First 8 bytes is u64 size
                let ciphertext = data[8..].to_vec();
                let header_start: usize = ciphertext.len() - size - 12;
                let nonce_start: usize = header_start + size;

                let header = Header::from(&ciphertext[..header_start]);
                let encrypted = &ciphertext[header_start..nonce_start];
                let nonce: [u8; 12] = ciphertext[nonce_start..].try_into().unwrap();
                debug!(header=hex::encode(Vec::<u8>::from(header.clone())),
                       encrypted=hex::encode(encrypted),
                       nonce=hex::encode(nonce),
                       "decryption started");
                let decrypted = ratchet.decrypt(&header, encrypted, &nonce, &self.associated_data);
                return Ok(decrypted);
            }
            None => { return Err(CryptoError::MissingRatchet); }
        }
    }
}