//! panosync: end-to-end encrypted clipboard sync.
//!
//! - [`crypto`]: key derivation and encryption
//! - [`protocol`]: the encrypted message payload and the checks on incoming messages
//! - [`net`]: the connection to the relay
//! - [`pause`]: the `pause` command's state file
//!
//! The command line, config file and clipboard polling live in the binary (`src/main.rs`).

pub mod crypto;
pub mod net;
pub mod pause;
pub mod protocol;

/// Current Unix time in seconds (0 if the clock is before 1970).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
