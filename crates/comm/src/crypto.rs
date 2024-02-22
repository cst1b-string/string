//! Handles the Double-Ratchet (DR) key exchange for communications

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use double_ratchet_rs::{Header, Ratchet};
use protocol::crypto;
use rand::rngs::OsRng;
use std::{collections::HashMap, fmt, io::Cursor};
use thiserror::Error;
use tracing::debug;
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Error, Debug)]
pub enum DoubleRatchetError {
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

/// Key exchange process happens as follows:

/// 1.   Initiator (Alice)  --[Alice's DH pubkey]-->  Responder (Bob)
///             |                                             |
///    [Alice's DH privkey]                          [Bob's DH privkey]
///
/// 2. Responder generates DH shared secret, SK
///
/// 3. Responder calls init_bob on SK to get DR pubkey, transitions to AlmostInitialized.
///    Bob is initialized at this point, but has not sent out its public keys
///
/// 4.   Initiator (Alice)   <--[Bob's DH pubkey]--   AlmostInitialized (Bob)
///             |               [Bob's DR pubkey]             |
///             |                                             |
///    [Alice's DH privkey]                          [Bob's DR ratchet]
///                                                  [Bob's DR pubkey ]
///
/// 5. Bob is already initialized and sent out its public keys, so now it transitions to Initialized
///
/// 6. Alice generates DH shared secret, SK
///
/// 7. Alice calls init_alice on SK and Bob's DR pubkey to get its own ratchet.
///    Both sides are now initialized.
///
/// 8. Alice sends any encrypted message to Bob so Bob's ratchet can encrypt too

/// An enum to handle the Double-Ratchet (DR) key exchange for communications.
pub enum DoubleRatchet {
    /// An initiator of a key exchange.
    Initiator { dh_privkey: StaticSecret },
    /// A responder to a key exchange.
    Responder { dh_privkey: StaticSecret },
    /// Intermediate state to complete key exchange
    /// When we call init_bob, it returns a DR pubkey and completes the ratchet
    /// We need to send this DR pubkey back to Alice before counting
    /// the ratchet as full initialized, hence the need for this state
    AlmostInitialized {
        ratchet: Ratchet,
        dh_privkey: StaticSecret,
        dr_pubkey: PublicKey,
    },
    /// An initialized DoubleRatchet object, with associated data.
    Initialized {
        ratchet: Ratchet,
        associated_data: Vec<u8>,
    },
}

pub struct Crypto {
    pub ratchets: HashMap<String, DoubleRatchet>,
}

impl Crypto {
    pub fn new() -> Self {
        Self {
            ratchets: HashMap::new(),
        }
    }
}

impl Default for Crypto {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DoubleRatchet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Opaque DoubleRatchet object>")
    }
}

impl fmt::Debug for Crypto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Opaque Crypto object>")
    }
}

impl DoubleRatchet {
    /// Create a new ratchet, with the given initiator status.
    pub fn new(is_initiator: bool) -> Self {
        if is_initiator {
            Self::new_initiator()
        } else {
            Self::new_responder()
        }
    }

    /// Create a new [DoubleRatchet] instance as an initiator.
    pub fn new_initiator() -> Self {
        Self::Initiator {
            dh_privkey: StaticSecret::random_from_rng(OsRng),
        }
    }

    /// Create a new [DoubleRatchet] instance as a responder.
    pub fn new_responder() -> Self {
        Self::Responder {
            dh_privkey: StaticSecret::random_from_rng(OsRng),
        }
    }

    /// Handle a key exchange packet, updating the internal state of the [DoubleRatchet] instance.
    pub fn handle_kex(
        &mut self,
        packet: crypto::v1::DrKeyExchange,
    ) -> Result<(), DoubleRatchetError> {
        // ensure we are not already initialized
        let dh_privkey = match self {
            DoubleRatchet::Initiator { dh_privkey } => dh_privkey,
            DoubleRatchet::Responder { dh_privkey } => dh_privkey,
            DoubleRatchet::AlmostInitialized { .. } => return Err(DoubleRatchetError::NonKexFail),
            DoubleRatchet::Initialized { .. } => return Err(DoubleRatchetError::NonKexFail),
        };

        let peer_dh_pubkey_bytes: [u8; 32] = packet.dh_pubkey[..32].try_into().unwrap();
        let peer_dh_pubkey = PublicKey::from(peer_dh_pubkey_bytes);
        let shared_secret = dh_privkey.diffie_hellman(&peer_dh_pubkey);

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

            *self = DoubleRatchet::AlmostInitialized {
                ratchet,
                dh_privkey: dh_privkey.clone(),
                dr_pubkey,
            }
        } else {
            let peer_dr_pubkey_bytes: [u8; 32] = packet.dr_pubkey[..32].try_into().unwrap();
            let peer_dr_pubkey = PublicKey::from(peer_dr_pubkey_bytes);
            debug!(
                pubkey = hex::encode(peer_dr_pubkey.clone().as_bytes()),
                "init_alice pubkey"
            );
            let ratchet = Ratchet::init_alice(shared_secret.to_bytes(), peer_dr_pubkey);

            *self = DoubleRatchet::Initialized {
                ratchet,
                associated_data: vec![],
            }
        }
        Ok(())
    }

    /// Create a new key exchange packet.
    pub fn generate_kex_message(&mut self) -> crypto::v1::DrKeyExchange {
        // if the ratchet is not initialised, we cannot decrypt
        let (dh_privkey, dr_pubkey) = match self {
            DoubleRatchet::Initiator { dh_privkey } => (dh_privkey, None),
            DoubleRatchet::AlmostInitialized {
                ratchet: _,
                dh_privkey,
                dr_pubkey,
            } => (dh_privkey, Some(dr_pubkey)),
            DoubleRatchet::Responder { .. } => {
                panic!("Should not initiate with Responder");
            }
            DoubleRatchet::Initialized { .. } => {
                panic!("Ratchet should not be initialised at this point");
            }
        };

        // get public keys in raw bytes
        let dh_pubkey_raw = PublicKey::from(&*dh_privkey).as_bytes().to_vec();
        let dr_pubkey_raw = match dr_pubkey {
            Some(d) => d.as_bytes().to_vec(),
            None => vec![],
        };

        match self {
            DoubleRatchet::AlmostInitialized { ratchet, .. } => {
                *self = DoubleRatchet::Initialized {
                    ratchet: Ratchet::import(&ratchet.export()).unwrap(),
                    associated_data: vec![],
                }
            }
            DoubleRatchet::Initiator { .. }
            | DoubleRatchet::Responder { .. }
            | DoubleRatchet::Initialized { .. } => {}
        };

        // create a new packet
        crypto::v1::DrKeyExchange {
            dh_pubkey: dh_pubkey_raw,
            dr_pubkey: dr_pubkey_raw,
        }
    }

    /// Encrypt the data using the ratchet, advancing the ratchet state in the process.
    pub fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, DoubleRatchetError> {
        // if the ratchet is not initialised, we cannot decrypt
        let (ratchet, associated_data) = match self {
            DoubleRatchet::Initiator { .. }
            | DoubleRatchet::Responder { .. }
            | DoubleRatchet::AlmostInitialized { .. } => {
                return Err(DoubleRatchetError::MissingRatchet)
            }
            DoubleRatchet::Initialized {
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
    pub fn decrypt(&mut self, data: &Vec<u8>) -> Result<Vec<u8>, DoubleRatchetError> {
        // if the ratchet is not initialised, we cannot decrypt
        let (ratchet, associated_data) = match self {
            DoubleRatchet::Initiator { .. }
            | DoubleRatchet::Responder { .. }
            | DoubleRatchet::AlmostInitialized { .. } => {
                return Err(DoubleRatchetError::MissingRatchet)
            }
            DoubleRatchet::Initialized {
                ratchet,
                associated_data,
                ..
            } => (ratchet, associated_data),
        };

        let mut cursor = Cursor::new(data);
        let size: usize = match cursor.read_u64::<BigEndian>() {
            Ok(s) => s.try_into().unwrap(),
            Err(_) => {
                return Err(DoubleRatchetError::BadCiphertext);
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
