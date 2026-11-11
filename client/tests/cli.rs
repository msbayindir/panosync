//! Runs the `panosync` binary with a temporary config directory (`PANOSYNC_CONFIG_DIR`).
//! None of these commands touch the clipboard or the network.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "panosync-cli-{}-{nanos}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn panosync(config_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_panosync"))
        .args(args)
        .env("PANOSYNC_CONFIG_DIR", config_dir)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn setup_then_pair_a_second_device() {
    let laptop = TempDir::new("laptop");
    let desktop = TempDir::new("desktop");

    let out = stdout(&panosync(
        &laptop.0,
        &["setup", "--server", "wss://relay.example.com"],
    ));
    assert!(out.contains("A new key was generated"));
    let key = stdout(&panosync(&laptop.0, &["key"])).trim().to_string();
    assert_eq!(key.len(), 43);
    assert!(out.contains(&format!(
        "panosync setup --server wss://relay.example.com --key {key}"
    )));

    // The Turkish aliases still work.
    let out = stdout(&panosync(
        &desktop.0,
        &[
            "kurulum",
            "--sunucu",
            "wss://relay.example.com",
            "--anahtar",
            &key,
        ],
    ));
    assert!(!out.contains("A new key was generated"));
    assert_eq!(stdout(&panosync(&desktop.0, &["anahtar"])).trim(), key);

    let config = std::fs::read_to_string(desktop.0.join("config.toml")).unwrap();
    assert!(config.contains("server = \"wss://relay.example.com\""));
    assert!(config.contains(&format!("key = \"{key}\"")));
    assert_eq!(
        stdout(&panosync(&desktop.0, &["config"])).trim(),
        desktop.0.join("config.toml").display().to_string()
    );
}

#[cfg(unix)]
#[test]
fn config_file_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("perms");
    stdout(&panosync(
        &dir.0,
        &["setup", "--server", "wss://relay.example.com"],
    ));
    let mode = std::fs::metadata(dir.0.join("config.toml"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn setup_rejects_bad_input() {
    let dir = TempDir::new("bad");
    assert!(
        !panosync(&dir.0, &["setup", "--server", "https://relay.example.com"])
            .status
            .success()
    );
    assert!(
        !panosync(
            &dir.0,
            &[
                "setup",
                "--server",
                "wss://relay.example.com",
                "--key",
                "short"
            ]
        )
        .status
        .success()
    );
    assert!(!dir.0.join("config.toml").exists());
    // Without a config, `key` explains what to do.
    let out = panosync(&dir.0, &["key"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Run `panosync setup` first"));
}

#[test]
fn pause_and_resume() {
    let dir = TempDir::new("pause");
    let pause_file = dir.0.join("paused_until");

    assert!(stdout(&panosync(&dir.0, &["pause", "5"])).contains("Paused for 5 minute(s)"));
    let until: u64 = std::fs::read_to_string(&pause_file)
        .unwrap()
        .parse()
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!((now + 290..=now + 310).contains(&until));

    assert!(stdout(&panosync(&dir.0, &["duraklat", "0"])).contains("Resumed"));
    assert!(!pause_file.exists());
}
