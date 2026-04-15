//! Privacy-preserving IP hashing for `sessions.ip_hash`.
//!
//! Raw IP addresses are never stored. We salt with a server-wide secret
//! *plus* the current date so that yesterday's hashes can't be reversed via
//! a rainbow table built today. The 48-hour deletion requirement in
//! `CLAUDE.md` §9 is implemented by a future cron sweep (not in Phase 1).

use std::net::IpAddr;

use chrono::NaiveDate;
use sha2::{Digest, Sha256};

/// Produce a 32-char hex digest for `(ip, salt, day)`.
pub fn hash_ip(ip: IpAddr, salt: &str, day: NaiveDate) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ip.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(salt.as_bytes());
    hasher.update(b"|");
    hasher.update(day.to_string().as_bytes());
    let digest = hasher.finalize();
    // 16 bytes → 32 hex chars. Enough to deduplicate, too short to meaningfully
    // reverse-engineer by brute force without the salt.
    hex::encode(&digest[..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn hash_is_stable() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
        let a = hash_ip(ip, "salt", day);
        let b = hash_ip(ip, "salt", day);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn different_salt_different_hash() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
        assert_ne!(hash_ip(ip, "a", day), hash_ip(ip, "b", day));
    }

    #[test]
    fn different_day_different_hash() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 4, 16).unwrap();
        assert_ne!(hash_ip(ip, "salt", d1), hash_ip(ip, "salt", d2));
    }
}
