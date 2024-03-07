//! # Lighthouse Client
//!
//! This crate provides a client for the lighthouse service.

use base64::prelude::*;
use lighthouse_protocol::{
    GetNodeAddrPayload, GetNodeAddrResponse, ListPotentialPeersPayload, ListPotentialPeersResponse,
    RegisterNodeAddrPayload,
};
use pgp::{
    composed::SignedSecretKey,
    types::{KeyTrait, SecretKeyTrait},
};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    str::{from_utf8, FromStr},
};
use string_comm::crypto::{Crypto, SigningError};
use thiserror::Error;

/// An enumeration of errors that can occur when using the lighthouse client.
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum LighthouseClientError {
    /// An unknown error occurred.
    #[error("unknown error")]
    Unknown,
    /// A web request failed.
    #[error("failed to make web request")]
    RequestError(#[from] reqwest::Error),
    /// An error occured while processing a public key.
    #[error("failed to export pubkey")]
    PubKeyError(#[from] pgp::errors::Error),
    /// An error occured while signing.
    #[error("failed to create signature")]
    SigningError(#[from] SigningError),
    /// An error occured while decoding base64.
    #[error("failed to decode base64 info string")]
    Base64Error(#[from] base64::DecodeError),
    /// An invalid info string format was provided.
    #[error("invalid info string format")]
    InfoStringError,
}

async fn public_ip() -> Result<Ipv4Addr, LighthouseClientError> {
    Ok(Ipv4Addr::from_str(
        reqwest::get("https://ipv4.icanhazip.com")
            .await
            .expect("icanhazip down")
            .text()
            .await
            .expect("icanhazip gave bad response")
            .strip_suffix('\n')
            .expect("stripping suffix failed"),
    )
    .expect("icanhazip gave bad response"))
}

/// Register this node's address with a lighthouse server.
pub async fn register_node_address(
    lighthouse_url: &String,
    addr: Option<Ipv4Addr>,
    port: u16,
    secret_key: SignedSecretKey,
) -> Result<(), LighthouseClientError> {
    // fetch ip from STUN if not provided
    let addr = match addr {
        Some(ip) => (ip, port).into(),
        None => (public_ip().await?, port).into(),
    };

    let now: u32 = chrono::Utc::now().timestamp() as u32;

    // sign the data - include timestamp to prevent replay attacks
    let signature = hex::encode(Crypto::sign_data_static(
        &secret_key.clone(),
        &format!("{}-{}", port, now).into_bytes(),
    )?);
    let pubkey = secret_key
        .public_key()
        .sign(&secret_key, || "".to_string())?
        .to_armored_string(None)?;

    let client = reqwest::Client::new();
    client
        .post(format!("{}/nodes", lighthouse_url))
        .json(
            &(RegisterNodeAddrPayload {
                addr,
                public_key: pubkey,
                signature,
                timestamp: now,
            }),
        )
        .send()
        .await?
        .json::<()>()
        .await?;
    Ok(())
}

/// Attempt to look up an endpoint from a lighthouse server,
pub async fn get_node_address<F: AsRef<[u8]>>(
    lighthouse_url: &String,
    addr: Option<Ipv4Addr>,
    port: u16,
    fingerprint: F,
) -> Result<SocketAddr, LighthouseClientError> {
    // fetch ip from STUN if not provided
    let addr = match addr {
        Some(ip) => (ip, port).into(),
        None => (public_ip().await?, port).into(),
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "{}/nodes/{}",
            lighthouse_url,
            hex::encode(fingerprint)
        ))
        .json(&(GetNodeAddrPayload { addr }))
        .send()
        .await?
        .json::<GetNodeAddrResponse>()
        .await?;

    Ok(response.addr)
}

/// List potential peers that have recently attempted to find information about the node
/// with the given secret key.
pub async fn list_potential_peers(
    lighthouse_url: &String,
    secret_key: &SignedSecretKey,
) -> Result<HashMap<String, SocketAddr>, LighthouseClientError> {
    let timestamp: u32 = chrono::Utc::now().timestamp() as u32;
    let signature = hex::encode(Crypto::sign_data_static(
        &secret_key,
        &timestamp.to_le_bytes(),
    )?);

    let client = reqwest::Client::new();
    Ok(client
        .post(format!("{}/peers", lighthouse_url))
        .json(
            &(ListPotentialPeersPayload {
                signature,
                timestamp,
                fingerprint: hex::encode(secret_key.public_key().fingerprint()),
                public_key: secret_key
                    .public_key()
                    .sign(secret_key, || "".to_string())?
                    .to_armored_string(None)?,
            }),
        )
        .send()
        .await?
        .json::<ListPotentialPeersResponse>()
        .await?
        .addrs)
}

/// A struct to hold encoded information.
#[derive(Serialize, Deserialize)]
struct EncodedInfo {
    #[serde(rename = "f")]
    fingerprint: String,
    #[serde(rename = "l")]
    lighthouse_url: String,
}

/// Encode a fingerprint, lighthouse URL, and id into a base64 string.
pub fn encode_info_str<S: AsRef<str>>(fingerprint: S, lighthouse_url: S) -> String {
    let data = EncodedInfo {
        fingerprint: fingerprint.as_ref().to_owned(),
        lighthouse_url: lighthouse_url.as_ref().to_owned(),
    };
    BASE64_STANDARD.encode(serde_json::to_string(&data).unwrap().as_bytes())
}

/// Decode an info string.
pub fn decode_info_str(info_str: &String) -> Result<(String, String), LighthouseClientError> {
    let raw = BASE64_STANDARD.decode(info_str)?;
    let EncodedInfo {
        fingerprint,
        lighthouse_url,
    } = serde_json::from_str(from_utf8(&raw).map_err(|_| LighthouseClientError::InfoStringError)?)
        .map_err(|_| LighthouseClientError::InfoStringError)?;
    Ok((fingerprint, lighthouse_url))
}
