use base64::prelude::*;
use pgp::{composed::SignedSecretKey, types::SecretKeyTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    net::Ipv4Addr,
    str::{from_utf8, FromStr},
};
use string_comm::crypto::{Crypto, SigningError};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum LighthouseClientError {
    #[error("unknown error")]
    Unknown,
    #[error("failed to make web request")]
    RequestError(#[from] reqwest::Error),
    #[error("failed to export pubkey")]
    PubkeyError(#[from] pgp::errors::Error),
    #[error("failed to create signature")]
    SigningError(#[from] SigningError),
    #[error("failed to decode base64 info string")]
    Base64Error(#[from] base64::DecodeError),
    #[error("invalid info string format")]
    InfostrError,
}

#[derive(Serialize)]
struct RegisterEndpointPayload {
    endpoint: String,
    pubkey: String,
    signature: String,
    timestamp: u32,
}

#[derive(Serialize)]
struct LookupEndpointPayload {
    id: String,
    client: String,
    fingerprint: String,
}

#[derive(Serialize)]
struct ListConnPayload {
    id: String,
    signature: String,
    timestamp: u32,
}

#[derive(Deserialize)]
struct RegisterEndpointResponse {
    id: String,
}

#[derive(Deserialize)]
struct LookupEndpointResponse {
    endpoint: String,
}

#[derive(Deserialize)]
struct ListConnResponse {
    conns: Vec<(String, String)>,
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

pub async fn register_endpoint(
    lighthouse_url: &String,
    ip: Option<Ipv4Addr>,
    port: u16,
    secret_key: SignedSecretKey,
) -> Result<String, LighthouseClientError> {
    let ip_addr = match ip {
        Some(ip) => ip,
        None => public_ip().await?,
    };

    let now: u32 = chrono::Utc::now().timestamp() as u32;
    let endpoint = format!("{}:{}", ip_addr, port);
    let signature = hex::encode(Crypto::sign_data_static(
        &secret_key.clone(),
        &format!("{}-{}", endpoint, now).into_bytes(),
    )?);
    let pubkey = secret_key
        .public_key()
        .sign(&secret_key, || "".to_string())?
        .to_armored_string(None)?;

    let payload = RegisterEndpointPayload {
        endpoint,
        pubkey,
        signature,
        timestamp: now,
    };

    let client = reqwest::Client::new();
    Ok(client
        .post(format!("{}/register", lighthouse_url))
        .json(&payload)
        .send()
        .await?
        .json::<RegisterEndpointResponse>()
        .await?
        .id)
}

pub async fn lookup_endpoint(
    lighthouse_url: &String,
    id: String,
    ip: Option<Ipv4Addr>,
    port: u16,
    fingerprint: &[u8],
) -> Result<String, LighthouseClientError> {
    let ip_addr = match ip {
        Some(ip) => ip,
        None => public_ip().await?,
    };

    let payload = LookupEndpointPayload {
        id,
        client: format!("{}:{}", ip_addr, port),
        fingerprint: hex::encode(fingerprint),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/lookup", lighthouse_url))
        .json(&payload)
        .send()
        .await?
        .json::<LookupEndpointResponse>()
        .await?;

    Ok(resp.endpoint)
}

pub async fn list_conns(
    lighthouse_url: &String,
    id: String,
    secret_key: SignedSecretKey,
) -> Result<Vec<(String, String)>, LighthouseClientError> {
    let now: u32 = chrono::Utc::now().timestamp() as u32;
    let signature = hex::encode(Crypto::sign_data_static(
        &secret_key.clone(),
        &format!("{}-{}", id, now).into_bytes(),
    )?);

    let payload = ListConnPayload {
        id,
        signature,
        timestamp: now,
    };

    let client = reqwest::Client::new();
    Ok(client
        .post(format!("{}/listconns", lighthouse_url))
        .json(&payload)
        .send()
        .await?
        .json::<ListConnResponse>()
        .await?
        .conns)
}

pub fn encode_info_str(fingerprint: &String, lighthouse_url: &String, id: &String) -> String {
    let data = json!({
        "f": fingerprint,
        "i": id,
        "l": lighthouse_url
    });
    BASE64_STANDARD.encode(data.to_string().as_bytes())
}

pub fn decode_info_str(
    info_str: &String,
) -> Result<(String, String, String), LighthouseClientError> {
    let raw = BASE64_STANDARD.decode(info_str)?;
    let res: serde_json::Value =
        serde_json::from_str(from_utf8(&raw).map_err(|_| LighthouseClientError::InfostrError)?)
            .map_err(|_| LighthouseClientError::InfostrError)?;

    let fingerprint = res
        .get("f")
        .ok_or(LighthouseClientError::InfostrError)?
        .as_str()
        .ok_or(LighthouseClientError::InfostrError)?
        .to_string();
    let id = res
        .get("i")
        .ok_or(LighthouseClientError::InfostrError)?
        .as_str()
        .ok_or(LighthouseClientError::InfostrError)?
        .to_string();
    let lighthouse = res
        .get("l")
        .ok_or(LighthouseClientError::InfostrError)?
        .as_str()
        .ok_or(LighthouseClientError::InfostrError)?
        .to_string();
    Ok((fingerprint, lighthouse, id))
}
