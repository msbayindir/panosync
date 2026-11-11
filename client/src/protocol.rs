//! The v1 message payload and the checks applied to incoming messages.
//!
//! Each clipboard change becomes a JSON payload that is sealed with [`Keys::seal`]. On the way
//! in, a message is accepted only if it decrypts, has the expected protocol version, did not
//! come from this device (echo), has not been seen before (replay), is not older or newer than
//! five minutes (clock window) and is not too large.

use crate::crypto::{Keys, PROTOCOL_VERSION};
use anyhow::{Result, bail};
use chacha20poly1305::aead::{OsRng, rand_core::RngCore};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Longest text that is sent or accepted, in bytes.
pub const MAX_TEXT_BYTES: usize = 512 * 1024;
/// Messages whose timestamp differs from the local clock by more than this are rejected.
pub const MAX_CLOCK_SKEW_SECS: u64 = 300;
/// How many recent message ids are remembered for replay protection.
const SEEN_IDS_CAPACITY: usize = 512;

/// What is encrypted inside every message. The relay never sees any of these fields.
#[derive(Serialize, Deserialize)]
struct Payload {
    /// Protocol version, same as the first byte of the sealed message.
    v: u8,
    /// Random id of the sending device, chosen at start-up. Used to ignore our own messages.
    device: String,
    /// Random message id, for replay protection.
    id: u64,
    /// Unix time in seconds when the message was sent.
    time: u64,
    /// The clipboard text.
    text: String,
}

/// Turns clipboard text into sealed messages and back, for one running device.
pub struct Codec {
    keys: Keys,
    device_id: String,
    seen_ids: VecDeque<u64>,
}

impl Codec {
    /// Creates a codec with a new random device id.
    pub fn new(keys: Keys) -> Self {
        Self::with_device_id(keys, format!("{:016x}", OsRng.next_u64()))
    }

    pub fn with_device_id(keys: Keys, device_id: String) -> Self {
        Self {
            keys,
            device_id,
            seen_ids: VecDeque::with_capacity(SEEN_IDS_CAPACITY),
        }
    }

    pub fn channel_id(&self) -> &str {
        &self.keys.channel_id
    }

    /// Seals `text` into a message for the other devices.
    pub fn seal(&self, text: String) -> Vec<u8> {
        self.seal_at(text, crate::unix_now())
    }

    fn seal_at(&self, text: String, time: u64) -> Vec<u8> {
        let payload = Payload {
            v: PROTOCOL_VERSION,
            device: self.device_id.clone(),
            id: OsRng.next_u64(),
            time,
            text,
        };
        self.keys
            .seal(&serde_json::to_vec(&payload).expect("payload serializes to JSON"))
    }

    /// Opens and validates an incoming message. Returns `None` for our own messages.
    pub fn open(&mut self, sealed: &[u8]) -> Result<Option<String>> {
        let payload: Payload = serde_json::from_slice(&self.keys.open(sealed)?)?;
        if payload.v != PROTOCOL_VERSION {
            bail!("unsupported payload version: {}", payload.v);
        }
        if payload.device == self.device_id {
            return Ok(None);
        }
        if crate::unix_now().abs_diff(payload.time) > MAX_CLOCK_SKEW_SECS {
            bail!("message is too old, or device clocks are out of sync");
        }
        if !self.remember(payload.id) {
            bail!("duplicate message (replay)");
        }
        if payload.text.len() > MAX_TEXT_BYTES {
            bail!("text is too large");
        }
        Ok(Some(payload.text))
    }

    /// Records a message id. Returns `false` if it was already seen.
    fn remember(&mut self, id: u64) -> bool {
        if self.seen_ids.contains(&id) {
            return false;
        }
        if self.seen_ids.len() >= SEEN_IDS_CAPACITY {
            self.seen_ids.pop_front();
        }
        self.seen_ids.push_back(id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_master_key;

    /// Two codecs for the same master key, as on two devices.
    fn pair() -> (Codec, Codec) {
        let master = generate_master_key();
        let keys = || Keys::from_master_key(&master).unwrap();
        (
            Codec::with_device_id(keys(), "mac".into()),
            Codec::with_device_id(keys(), "linux".into()),
        )
    }

    #[test]
    fn message_reaches_the_other_device() {
        let (mac, mut linux) = pair();
        let sealed = mac.seal("hello".into());
        assert_eq!(linux.open(&sealed).unwrap().as_deref(), Some("hello"));
    }

    #[test]
    fn replayed_message_is_rejected() {
        let (mac, mut linux) = pair();
        let sealed = mac.seal("hello".into());
        assert!(linux.open(&sealed).unwrap().is_some());
        assert!(linux.open(&sealed).is_err());
    }

    #[test]
    fn own_message_is_ignored() {
        let (mut mac, _) = pair();
        let sealed = mac.seal("hello".into());
        assert!(mac.open(&sealed).unwrap().is_none());
    }

    #[test]
    fn old_and_future_messages_are_rejected() {
        let (mac, mut linux) = pair();
        let now = crate::unix_now();
        let too_old = mac.seal_at("old".into(), now - MAX_CLOCK_SKEW_SECS - 10);
        let too_new = mac.seal_at("new".into(), now + MAX_CLOCK_SKEW_SECS + 10);
        assert!(linux.open(&too_old).is_err());
        assert!(linux.open(&too_new).is_err());
    }

    #[test]
    fn oversized_text_is_rejected() {
        let (mac, mut linux) = pair();
        let sealed = mac.seal("x".repeat(MAX_TEXT_BYTES + 1));
        assert!(linux.open(&sealed).is_err());
    }

    #[test]
    fn payload_with_other_version_is_rejected() {
        let master = generate_master_key();
        let keys = Keys::from_master_key(&master).unwrap();
        let mut linux = Codec::new(Keys::from_master_key(&master).unwrap());
        let payload = Payload {
            v: PROTOCOL_VERSION + 1,
            device: "mac".into(),
            id: 1,
            time: crate::unix_now(),
            text: "hello".into(),
        };
        let sealed = keys.seal(&serde_json::to_vec(&payload).unwrap());
        assert!(linux.open(&sealed).is_err());
    }
}
