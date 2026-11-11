//! `panosync pause <minutes>`: stop sending clipboard changes for a while.
//!
//! The end of the pause (Unix time in seconds) is written to a small file in the config
//! directory. The running client reads it whenever the clipboard changes, so no inter-process
//! communication is needed and the command works whether or not the client is running.

use anyhow::Result;
use std::path::Path;

/// Name of the pause file inside the config directory.
pub const FILE_NAME: &str = "paused_until";

/// Pauses for `minutes` and returns the end time. `minutes == 0` removes the pause.
pub fn pause(file: &Path, minutes: u64, now: u64) -> Result<Option<u64>> {
    if minutes == 0 {
        match std::fs::remove_file(file) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        return Ok(None);
    }

    let until = now.saturating_add(minutes.saturating_mul(60));
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Write to a temporary file and rename it into place, so the client never reads a
    // half-written file and misses the pause.
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, until.to_string())?;
    std::fs::rename(&tmp, file)?;
    Ok(Some(until))
}

/// Returns the end time if a pause is in effect.
///
/// An unreadable or corrupt file counts as paused (fail closed): the user may have paused to
/// copy a password. `panosync pause 0` clears it.
pub fn paused_until(file: &Path, now: u64) -> Option<u64> {
    let contents = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => return Some(u64::MAX),
    };
    match contents.trim().parse::<u64>() {
        Ok(until) if until > now => Some(until),
        Ok(_) => None,
        Err(_) => Some(u64::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A pause file path in a fresh temporary directory, removed on drop.
    struct TempPauseFile(PathBuf);

    impl TempPauseFile {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "panosync-test-{}-{}-{name}",
                std::process::id(),
                crate::unix_now()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir.join(FILE_NAME))
        }
    }

    impl Drop for TempPauseFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    #[test]
    fn pause_lasts_for_the_given_minutes() {
        let file = TempPauseFile::new("duration");
        assert_eq!(paused_until(&file.0, 1_000), None);

        assert_eq!(pause(&file.0, 5, 1_000).unwrap(), Some(1_300));
        assert_eq!(paused_until(&file.0, 1_000), Some(1_300));
        assert_eq!(paused_until(&file.0, 1_299), Some(1_300));
        // Once the time is up the pause ends, even though the file is still there.
        assert_eq!(paused_until(&file.0, 1_300), None);
        assert_eq!(paused_until(&file.0, 5_000), None);
    }

    #[test]
    fn zero_minutes_resumes() {
        let file = TempPauseFile::new("zero");
        pause(&file.0, 10, 1_000).unwrap();
        assert!(paused_until(&file.0, 1_000).is_some());
        assert_eq!(pause(&file.0, 0, 1_000).unwrap(), None);
        assert_eq!(paused_until(&file.0, 1_000), None);
        // Resuming when not paused is not an error.
        assert_eq!(pause(&file.0, 0, 1_000).unwrap(), None);
    }

    #[test]
    fn corrupt_file_counts_as_paused() {
        let file = TempPauseFile::new("corrupt");
        std::fs::create_dir_all(file.0.parent().unwrap()).unwrap();
        std::fs::write(&file.0, "garbage").unwrap();
        assert_eq!(paused_until(&file.0, 1_000), Some(u64::MAX));
    }

    #[test]
    fn huge_duration_does_not_overflow() {
        let file = TempPauseFile::new("overflow");
        assert_eq!(pause(&file.0, u64::MAX, 1_000).unwrap(), Some(u64::MAX));
    }
}
