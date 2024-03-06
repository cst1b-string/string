//! This crate contains the protocol definitions for the Lighthouse network, and is usedf
//! by both the client and the server.

use std::{collections::HashMap, net::SocketAddr};

use nom::combinator::map;
use pgp::{
    crypto::hash::HashAlgorithm,
    types::{mpi, PublicKeyTrait},
    Deserializable, SignedPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A trait for signing and verifying payloads.
pub trait Sign {
    /// Returns the data to be signed.
    fn data(&self) -> Vec<u8>;

    /// Returns the signature of the payload.
    fn signature(&self) -> Vec<u8>;

    /// Returns the public key of the signer.
    fn public_key(&self) -> &String;

    /// Ensures that the signature is valid.
    fn verify(&self) -> Result<(), pgp::errors::Error> {
        // hash data?
        let digest = {
            let mut hasher = Sha256::new();
            hasher.update(self.data());
            hasher.finalize()
        };
        let digest = digest.as_slice();

        // do something?
        let signature = self.signature();
        let (_, mpi_signature) = map(mpi, |v| vec![v.to_owned()])(&signature)?;

        // access the public key
        let (public_key, _headers) = SignedPublicKey::from_string(&self.public_key())?;

        // verify
        public_key.verify_signature(HashAlgorithm::SHA2_256, digest, &mpi_signature)
    }
}

/// Used to register a node's address with the lighthouse server.
#[derive(Serialize, Deserialize)]
pub struct RegisterNodeAddrPayload {
    /// The address of the node, found via STUN.
    pub addr: SocketAddr,
    /// The public key of the node.
    pub public_key: String,
    /// A signature constructed from the public key and the address.
    pub signature: String,
    /// The timestamp of the request.
    pub timestamp: u32,
}

impl Sign for RegisterNodeAddrPayload {
    fn signature(&self) -> Vec<u8> {
        hex::decode(&self.signature).unwrap()
    }

    fn public_key(&self) -> &String {
        &self.public_key
    }

    fn data(&self) -> Vec<u8> {
        format!("{}-{}", self.public_key, self.addr)
            .as_bytes()
            .to_vec()
    }
}

/// The response to a node address registration.
#[derive(Serialize, Deserialize)]
pub struct RegisterNodeAddrResponse {}

/// Used to get the address of a node.
#[derive(Serialize, Deserialize)]
pub struct GetNodeAddrPayload {
    /// The address of the node making the request, found via STUN.
    pub addr: SocketAddr,
}

/// The response to a node address request.
#[derive(Serialize, Deserialize)]
pub struct GetNodeAddrResponse {
    /// The address of the node.
    pub addr: SocketAddr,
}

/// Used to list potential peers.
#[derive(Serialize, Deserialize)]
pub struct ListPotentialPeersPayload {
    /// The fingerprint of the node making the request.
    pub fingerprint: String,
    /// The public key of the node making the request.
    pub public_key: String,
    /// The signature of the payload, constructed from the secret key and the timestamp.
    pub signature: String,
    /// The timestamp of the request.
    pub timestamp: u32,
}

impl Sign for ListPotentialPeersPayload {
    fn signature(&self) -> Vec<u8> {
        hex::decode(&self.signature).unwrap()
    }

    fn public_key(&self) -> &String {
        &self.public_key
    }

    fn data(&self) -> Vec<u8> {
        self.timestamp.to_le_bytes().to_vec()
    }
}

/// The response to a potential peer list request.
#[derive(Serialize, Deserialize)]
pub struct ListPotentialPeersResponse {
    pub addrs: HashMap<String, SocketAddr>,
}
