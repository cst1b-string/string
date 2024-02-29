use std::{env, fs::File, io::Write};

use pgp::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::CompressionAlgorithm,
    Deserializable, KeyType, SecretKeyParamsBuilder, SignedSecretKey,
};
use smallvec::smallvec;

/// Generate a new PGP key.
fn generate_key(username: String, password: String) -> SignedSecretKey {
    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
        .key_type(KeyType::Rsa(2048))
        .can_certify(false)
        .can_sign(true)
        .primary_user_id(username.into())
        .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256])
        .preferred_hash_algorithms(smallvec![HashAlgorithm::SHA2_256])
        .preferred_compression_algorithms(smallvec![CompressionAlgorithm::ZLIB]);

    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");
    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");
    let passwd_fn = || password;
    let signed_secret_key = secret_key
        .sign(passwd_fn)
        .expect("Must be able to sign its own metadata");
    signed_secret_key
}

fn load_key(location: &String) -> Option<SignedSecretKey> {
    let Ok(mut file) = File::open(&location) else {
        return None;
    };
    let Ok((key, _headers)) = SignedSecretKey::from_armor_single(&mut file) else {
        return None;
    };
    Some(key)
}

fn save_key(location: &String, key: SignedSecretKey) {
    let mut file = File::create(location).expect("Error opening privkey file");
    file.write_all(
        key.to_armored_string(None)
            .expect("Error generating armored string")
            .as_bytes(),
    )
    .expect("Error writing privkey");
}

fn get_key_path() -> String {
    let cwd = env::current_dir().expect("Failed to get current dir");
    let cwd_str = cwd.to_str().expect("Failed to convert dir to string");
    format!("{cwd_str}/key.asc")
}
