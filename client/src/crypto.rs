//! Key derivation and encryption.
//!
//! A user's devices share one secret: a random 32-byte *master key*. Two independent values
//! are derived from it with BLAKE3 `derive_key`, each with its own context string:
//!
//! - the **encryption key**, used with XChaCha20-Poly1305 to seal every message;
//! - the **channel id**, which the relay uses to group the user's devices.
//!
//! Because the contexts differ, the channel id (which the relay sees) reveals nothing
//! about the encryption key, and it cannot be guessed without the master key.
//!
//! A sealed message is `[protocol version: 1 byte][nonce: 24 bytes][ciphertext + tag: 16 bytes]`.
//! See `docs/architecture.md` for the full v1 protocol.

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, OsRng, Payload},
};

/// Version of the wire protocol. It is the first byte of every sealed message and is also
/// carried inside the encrypted payload. Any change to the message format or to key
/// derivation must increase it.
pub const PROTOCOL_VERSION: u8 = 1;

const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const HEADER_LEN: usize = 1 + NONCE_LEN;

/// Associated data authenticated with every ciphertext.
const ASSOCIATED_DATA: &[u8] = b"panosync v1";
const ENCRYPTION_KEY_CONTEXT: &str = "panosync v1 encryption key";
const CHANNEL_ID_CONTEXT: &str = "panosync v1 channel id";

/// Everything derived from the master key.
pub struct Keys {
    cipher: XChaCha20Poly1305,
    /// 64 lowercase hex characters; the relay groups devices by this value.
    pub channel_id: String,
}

/// Generates a new random master key, encoded as unpadded URL-safe base64 (43 characters).
pub fn generate_master_key() -> String {
    URL_SAFE_NO_PAD.encode(XChaCha20Poly1305::generate_key(&mut OsRng))
}

impl Keys {
    /// Decodes a master key and derives the encryption key and the channel id from it.
    pub fn from_master_key(master_key: &str) -> Result<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(master_key.trim())
            .context("could not read the key; it may be incomplete or mistyped")?;
        let master: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("the key must be 32 bytes"))?;

        let encryption_key = blake3::derive_key(ENCRYPTION_KEY_CONTEXT, &master);
        let channel_id = blake3::derive_key(CHANNEL_ID_CONTEXT, &master);

        Ok(Self {
            cipher: XChaCha20Poly1305::new(&encryption_key.into()),
            channel_id: blake3::Hash::from(channel_id).to_hex().to_string(),
        })
    }

    /// Encrypts `plaintext` with a fresh random nonce.
    pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: ASSOCIATED_DATA,
                },
            )
            .expect("encryption cannot fail");

        let mut sealed = Vec::with_capacity(HEADER_LEN + ciphertext.len());
        sealed.push(PROTOCOL_VERSION);
        sealed.extend_from_slice(&nonce);
        sealed.extend_from_slice(&ciphertext);
        sealed
    }

    /// Decrypts a sealed message. Fails if it was modified, truncated, sealed with another
    /// key or uses another protocol version.
    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>> {
        if sealed.len() < HEADER_LEN + TAG_LEN {
            bail!("message is too short");
        }
        if sealed[0] != PROTOCOL_VERSION {
            bail!("unsupported protocol version: {}", sealed[0]);
        }
        let nonce = XNonce::from_slice(&sealed[1..HEADER_LEN]);
        self.cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &sealed[HEADER_LEN..],
                    aad: ASSOCIATED_DATA,
                },
            )
            .map_err(|_| anyhow::anyhow!("decryption failed (wrong key or corrupted message)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_then_open() {
        let keys = Keys::from_master_key(&generate_master_key()).unwrap();
        let sealed = keys.seal(b"hello world");
        assert!(!sealed.windows(5).any(|w| w == b"hello"));
        assert_eq!(sealed[0], PROTOCOL_VERSION);
        assert_eq!(keys.open(&sealed).unwrap(), b"hello world");
    }

    #[test]
    fn wrong_key_cannot_open() {
        let a = Keys::from_master_key(&generate_master_key()).unwrap();
        let b = Keys::from_master_key(&generate_master_key()).unwrap();
        assert!(b.open(&a.seal(b"secret")).is_err());
        assert_ne!(a.channel_id, b.channel_id);
    }

    #[test]
    fn modified_message_is_rejected() {
        let keys = Keys::from_master_key(&generate_master_key()).unwrap();
        let mut sealed = keys.seal(b"secret");
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert!(keys.open(&sealed).is_err());
    }

    #[test]
    fn other_protocol_version_is_rejected() {
        let keys = Keys::from_master_key(&generate_master_key()).unwrap();
        let mut sealed = keys.seal(b"secret");
        sealed[0] = PROTOCOL_VERSION + 1;
        assert!(keys.open(&sealed).is_err());
    }

    #[test]
    fn same_master_key_gives_same_channel() {
        let master = generate_master_key();
        let a = Keys::from_master_key(&master).unwrap();
        let b = Keys::from_master_key(&master).unwrap();
        assert_eq!(a.channel_id, b.channel_id);
        assert_eq!(a.channel_id.len(), 64);
    }

    /// Pins the v1 key derivation. If this test fails, the change breaks compatibility
    /// with every existing device and must come with a new protocol version.
    #[test]
    fn v1_derivation_is_stable() {
        let master = URL_SAFE_NO_PAD.encode([7u8; 32]);
        let keys = Keys::from_master_key(&master).unwrap();
        assert_eq!(keys.channel_id, V1_CHANNEL_ID_FOR_SEVENS);
    }

    /// Channel id derived from a master key of 32 bytes of 0x07 (documented in docs/architecture.md).
    const V1_CHANNEL_ID_FOR_SEVENS: &str =
        "5c08f796719c0a8c00c0a4ad475932a56bac3d55d5491d8a9823475c80519236";
}
