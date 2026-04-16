//! RSA key generation and management for ActivityPub HTTP Signatures.
//!
//! Each Vonk user gets a 2048-bit RSA keypair generated on first
//! access to their actor endpoint. The public key is shared in the
//! Person object; the private key signs outgoing HTTP requests.

use anyhow::{Context, Result};
use pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::RsaPrivateKey;
use sqlx::PgPool;

/// Generate a fresh 2048-bit RSA keypair and return `(public_pem, private_pem)`.
pub fn generate_keypair() -> Result<(String, String)> {
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048)
        .context("generating RSA private key")?;

    let private_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .context("encoding private key to PEM")?
        .to_string();

    let public_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .context("encoding public key to PEM")?;

    Ok((public_pem, private_pem))
}

/// Ensure the given user has an RSA keypair. If `ap_pubkey` is NULL,
/// generates one and stores it. Returns the public key PEM.
pub async fn ensure_keypair(db: &PgPool, user_id: i64) -> Result<String> {
    // Check if already present.
    let existing: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT ap_pubkey FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .context("checking existing keypair")?;

    if let Some((Some(pubkey),)) = existing {
        return Ok(pubkey);
    }

    // Generate and store.
    let (public_pem, private_pem) = generate_keypair()?;

    sqlx::query(
        "UPDATE users SET ap_pubkey = $1, ap_privkey = $2 WHERE id = $3",
    )
    .bind(&public_pem)
    .bind(&private_pem)
    .execute(db)
    .await
    .context("storing RSA keypair")?;

    Ok(public_pem)
}

/// Load the user's RSA private key PEM from the database.
/// Used by the delivery worker to sign outgoing HTTP requests.
#[allow(dead_code)]
pub async fn load_private_key(db: &PgPool, user_id: i64) -> Result<RsaPrivateKey> {
    let (pem,): (String,) = sqlx::query_as(
        "SELECT ap_privkey FROM users WHERE id = $1 AND ap_privkey IS NOT NULL",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .context("loading private key")?;

    RsaPrivateKey::from_pkcs8_pem(&pem).context("parsing private key PEM")
}
