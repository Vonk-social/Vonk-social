//! HTTP Signature verification and signing for ActivityPub.
//!
//! Implements a subset of the HTTP Signatures spec (draft-cavage-http-signatures)
//! used by Mastodon and other fediverse software:
//! - Sign outgoing POST requests with the user's RSA private key
//! - Verify incoming request signatures against the remote actor's public key

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

/// Sign an outgoing HTTP request body and return the headers needed.
///
/// Returns a vec of `(header_name, header_value)` pairs to set on the request:
/// `Date`, `Digest`, `Signature`.
/// Used by the delivery worker to send activities to remote inboxes.
#[allow(dead_code)]
pub fn sign_request(
    private_key: &RsaPrivateKey,
    key_id: &str,
    method: &str,
    path: &str,
    host: &str,
    body: &[u8],
) -> Result<Vec<(&'static str, String)>> {
    let date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

    // Body digest (SHA-256).
    let digest = {
        let mut hasher = Sha256::new();
        hasher.update(body);
        let hash = hasher.finalize();
        format!(
            "SHA-256={}",
            base64::engine::general_purpose::STANDARD.encode(hash)
        )
    };

    // Construct the signing string.
    let signing_string = format!(
        "(request-target): {} {}\nhost: {}\ndate: {}\ndigest: {}",
        method.to_lowercase(),
        path,
        host,
        date,
        digest,
    );

    // RSA-SHA256 signature.
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let signature = signing_key.sign(signing_string.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    let sig_header = format!(
        r#"keyId="{}",algorithm="rsa-sha256",headers="(request-target) host date digest",signature="{}""#,
        key_id, sig_b64,
    );

    Ok(vec![
        ("Date", date),
        ("Digest", digest),
        ("Signature", sig_header),
    ])
}

/// Verify an incoming HTTP Signature against the remote actor's public key.
///
/// Fetches the remote actor's public key if not already cached. Returns
/// the actor URI on success.
pub async fn verify_incoming(
    db: &PgPool,
    http: &reqwest::Client,
    method: &str,
    path: &str,
    headers: &axum::http::HeaderMap,
) -> Result<String> {
    // Parse the Signature header.
    let sig_header = headers
        .get("signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow!("missing Signature header"))?;

    let parsed = parse_signature_header(sig_header)?;

    // Fetch the remote actor's public key.
    let public_key_pem = fetch_actor_public_key(db, http, &parsed.key_id).await?;
    let public_key =
        RsaPublicKey::from_public_key_pem(&public_key_pem).context("parsing remote public key")?;

    // Reconstruct the signing string from the headers listed.
    let signing_string = build_signing_string(method, path, headers, &parsed.signed_headers)?;

    // Verify the signature.
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::signature::Verifier;

    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(&parsed.signature)
        .context("decoding signature base64")?;
    let signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
        .map_err(|e| anyhow!("invalid signature bytes: {}", e))?;

    verifying_key
        .verify(signing_string.as_bytes(), &signature)
        .map_err(|_| anyhow!("signature verification failed"))?;

    // Extract the actor URI from the key_id (format: "https://example.com/users/alice#main-key").
    let actor_uri = parsed
        .key_id
        .split('#')
        .next()
        .unwrap_or(&parsed.key_id)
        .to_string();

    Ok(actor_uri)
}

// ── Internal helpers ────────────────────────────────────────

struct ParsedSignature {
    key_id: String,
    signed_headers: Vec<String>,
    signature: String,
}

fn parse_signature_header(header: &str) -> Result<ParsedSignature> {
    let mut key_id = None;
    let mut headers_list = None;
    let mut signature = None;

    for part in header.split(',') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("keyId=") {
            key_id = Some(val.trim_matches('"').to_string());
        } else if let Some(val) = part.strip_prefix("headers=") {
            headers_list = Some(
                val.trim_matches('"')
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            );
        } else if let Some(val) = part.strip_prefix("signature=") {
            signature = Some(val.trim_matches('"').to_string());
        }
    }

    Ok(ParsedSignature {
        key_id: key_id.ok_or_else(|| anyhow!("missing keyId in Signature"))?,
        signed_headers: headers_list.unwrap_or_else(|| vec!["date".to_string()]),
        signature: signature.ok_or_else(|| anyhow!("missing signature in Signature"))?,
    })
}

fn build_signing_string(
    method: &str,
    path: &str,
    headers: &axum::http::HeaderMap,
    signed_headers: &[String],
) -> Result<String> {
    let mut parts = Vec::new();
    for h in signed_headers {
        let line = if h == "(request-target)" {
            format!("(request-target): {} {}", method.to_lowercase(), path)
        } else {
            let val = headers
                .get(h.as_str())
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow!("missing signed header: {}", h))?;
            format!("{}: {}", h, val)
        };
        parts.push(line);
    }
    Ok(parts.join("\n"))
}

/// Fetch a remote actor's public key PEM. Checks the local cache first,
/// then fetches from the remote server.
async fn fetch_actor_public_key(
    db: &PgPool,
    http: &reqwest::Client,
    key_id: &str,
) -> Result<String> {
    // key_id is typically "https://example.com/users/alice#main-key"
    let actor_uri = key_id.split('#').next().unwrap_or(key_id);

    // Check cache.
    let cached: Option<(String,)> = sqlx::query_as(
        "SELECT public_key_pem FROM ap_remote_actors WHERE actor_uri = $1",
    )
    .bind(actor_uri)
    .fetch_optional(db)
    .await
    .context("checking remote actor cache")?;

    if let Some((pem,)) = cached {
        return Ok(pem);
    }

    // Fetch the remote actor document.
    let resp = http
        .get(actor_uri)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .context("fetching remote actor")?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "remote actor fetch failed: {} {}",
            resp.status(),
            actor_uri
        ));
    }

    let actor_json: serde_json::Value = resp.json().await.context("parsing remote actor JSON")?;

    let public_key_pem = actor_json
        .get("publicKey")
        .and_then(|pk| pk.get("publicKeyPem"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("remote actor missing publicKey.publicKeyPem"))?
        .to_string();

    let inbox_url = actor_json
        .get("inbox")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let shared_inbox_url = actor_json
        .get("endpoints")
        .and_then(|e| e.get("sharedInbox"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let username = actor_json
        .get("preferredUsername")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let display_name = actor_json
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let avatar_url = actor_json
        .get("icon")
        .and_then(|i| i.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let summary = actor_json
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Cache the remote actor.
    sqlx::query(
        "INSERT INTO ap_remote_actors (actor_uri, inbox_url, shared_inbox_url, public_key_pem, \
         username, display_name, avatar_url, summary) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (actor_uri) DO UPDATE SET \
           public_key_pem = EXCLUDED.public_key_pem, \
           inbox_url = EXCLUDED.inbox_url, \
           shared_inbox_url = EXCLUDED.shared_inbox_url, \
           username = EXCLUDED.username, \
           display_name = EXCLUDED.display_name, \
           avatar_url = EXCLUDED.avatar_url, \
           summary = EXCLUDED.summary, \
           fetched_at = now()",
    )
    .bind(actor_uri)
    .bind(&inbox_url)
    .bind(&shared_inbox_url)
    .bind(&public_key_pem)
    .bind(&username)
    .bind(&display_name)
    .bind(&avatar_url)
    .bind(&summary)
    .execute(db)
    .await
    .context("caching remote actor")?;

    Ok(public_key_pem)
}
