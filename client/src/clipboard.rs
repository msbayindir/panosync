//! Clipboard polling.
//!
//! Runs on its own thread, because the OS clipboard APIs are blocking and some platforms
//! expect them to be used from a single thread. Every 400 ms it writes text received from
//! other devices to the clipboard, then checks whether the local clipboard changed and, if so,
//! hands the new text to the network side, unless the change is an echo of received text,
//! too large, or sync is paused.

use panosync::{pause, protocol::MAX_TEXT_BYTES, unix_now};
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc::{self, error::TryRecvError};

const POLL_INTERVAL: Duration = Duration::from_millis(400);

/// Polls the clipboard until the network side goes away.
pub fn run(
    outgoing: mpsc::Sender<String>,
    mut incoming: mpsc::Receiver<String>,
    pause_file: PathBuf,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let mut clipboard = arboard::Clipboard::new().context("could not access the clipboard")?;
    // Whatever is on the clipboard at start-up is not sent.
    let mut tracker = ChangeTracker::new(clipboard.get_text().ok());

    loop {
        if !apply_incoming(&mut clipboard, &mut incoming, &mut tracker) {
            return Ok(());
        }

        if let Ok(text) = clipboard.get_text() {
            let is_paused = || pause::paused_until(&pause_file, unix_now()).is_some();
            match tracker.local_change(text, is_paused) {
                LocalChange::Unchanged => {}
                LocalChange::Paused => tracing::info!("paused, clipboard change not sent"),
                LocalChange::TooLarge(len) => {
                    tracing::warn!("copied text is too large ({len} bytes), not sent")
                }
                LocalChange::Send(text) => {
                    if outgoing.try_send(text).is_err() {
                        tracing::warn!("send queue is full, this copy was skipped");
                    }
                }
            }
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Writes all text received from other devices to the clipboard.
/// Returns `false` once the network side has shut down.
fn apply_incoming(
    clipboard: &mut arboard::Clipboard,
    incoming: &mut mpsc::Receiver<String>,
    tracker: &mut ChangeTracker,
) -> bool {
    loop {
        match incoming.try_recv() {
            Ok(text) => match clipboard.set_text(text.as_str()) {
                Ok(()) => tracker.received(text),
                Err(e) => tracing::warn!("could not write to the clipboard: {e}"),
            },
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => return false,
        }
    }
}

#[derive(Debug, PartialEq)]
enum LocalChange {
    Unchanged,
    Paused,
    TooLarge(usize),
    Send(String),
}

/// Remembers the last clipboard text seen or written, to detect real changes.
struct ChangeTracker {
    last: Option<String>,
}

impl ChangeTracker {
    fn new(initial: Option<String>) -> Self {
        Self { last: initial }
    }

    /// Records text written to the clipboard from another device, so that it is not mistaken
    /// for a local copy and sent back (echo).
    fn received(&mut self, text: String) {
        self.last = Some(text);
    }

    /// Decides what to do with the current clipboard text. `is_paused` is only called when the
    /// text actually changed.
    fn local_change(&mut self, current: String, is_paused: impl FnOnce() -> bool) -> LocalChange {
        if current.is_empty() || self.last.as_deref() == Some(current.as_str()) {
            return LocalChange::Unchanged;
        }
        // Remembered even when paused or too large, so that it is not sent later
        // (e.g. a password copied during a pause).
        self.last = Some(current.clone());
        if is_paused() {
            LocalChange::Paused
        } else if current.len() > MAX_TEXT_BYTES {
            LocalChange::TooLarge(current.len())
        } else {
            LocalChange::Send(current)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAUSED: fn() -> bool = || true;
    const RUNNING: fn() -> bool = || false;

    #[test]
    fn initial_clipboard_is_not_sent() {
        let mut tracker = ChangeTracker::new(Some("already there".into()));
        assert_eq!(
            tracker.local_change("already there".into(), RUNNING),
            LocalChange::Unchanged
        );
        assert_eq!(
            tracker.local_change("new".into(), RUNNING),
            LocalChange::Send("new".into())
        );
        assert_eq!(
            tracker.local_change("new".into(), RUNNING),
            LocalChange::Unchanged
        );
    }

    #[test]
    fn received_text_is_not_sent_back() {
        let mut tracker = ChangeTracker::new(None);
        tracker.received("from the other device".into());
        assert_eq!(
            tracker.local_change("from the other device".into(), RUNNING),
            LocalChange::Unchanged
        );
    }

    #[test]
    fn text_copied_while_paused_is_never_sent() {
        let mut tracker = ChangeTracker::new(None);
        assert_eq!(
            tracker.local_change("hunter2".into(), PAUSED),
            LocalChange::Paused
        );
        // After the pause ends, the same clipboard content is still not sent.
        assert_eq!(
            tracker.local_change("hunter2".into(), RUNNING),
            LocalChange::Unchanged
        );
        assert_eq!(
            tracker.local_change("next".into(), RUNNING),
            LocalChange::Send("next".into())
        );
    }

    #[test]
    fn empty_and_oversized_text() {
        let mut tracker = ChangeTracker::new(None);
        assert_eq!(
            tracker.local_change(String::new(), RUNNING),
            LocalChange::Unchanged
        );
        let big = "x".repeat(MAX_TEXT_BYTES + 1);
        assert_eq!(
            tracker.local_change(big.clone(), RUNNING),
            LocalChange::TooLarge(big.len())
        );
    }

    #[test]
    fn pause_file_is_only_checked_on_change() {
        let mut tracker = ChangeTracker::new(Some("same".into()));
        let must_not_be_called = || -> bool { panic!("checked the pause file without a change") };
        assert_eq!(
            tracker.local_change("same".into(), must_not_be_called),
            LocalChange::Unchanged
        );
    }
}
