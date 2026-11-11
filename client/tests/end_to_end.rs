//! Starts a real relay and connects devices to it (channels stand in for the clipboard).
//!
//! Set `PANOSYNC_TEST_RELAY=wss://relay.example.com` to run the same tests against an
//! already running relay instead, e.g. to check a deployment or a reverse proxy.

use panosync::{
    crypto::{Keys, generate_master_key},
    net,
};
use std::time::Duration;
use tokio::{sync::mpsc, time::timeout};

async fn relay_address() -> String {
    if let Ok(external) = std::env::var("PANOSYNC_TEST_RELAY") {
        let _ = rustls::crypto::ring::default_provider().install_default();
        return external;
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(panosync_relay::serve(listener));
    format!("ws://{address}")
}

/// One simulated device: send text with `copy`, receive with `paste`.
struct Device {
    copy: mpsc::Sender<String>,
    paste: mpsc::Receiver<String>,
}

fn device(server: &str, master_key: &str) -> Device {
    let (copy, outgoing) = mpsc::channel(16);
    let (incoming, paste) = mpsc::channel(16);
    let keys = Keys::from_master_key(master_key).unwrap();
    tokio::spawn(net::run(server.to_string(), keys, outgoing, incoming));
    Device { copy, paste }
}

async fn next(device: &mut Device) -> Option<String> {
    timeout(Duration::from_secs(5), device.paste.recv())
        .await
        .ok()
        .flatten()
}

#[tokio::test]
async fn text_travels_between_devices_with_the_same_key() {
    let server = relay_address().await;
    let key = generate_master_key();
    let mut mac = device(&server, &key);
    let mut linux = device(&server, &key);
    let mut stranger = device(&server, &generate_master_key());
    tokio::time::sleep(Duration::from_millis(1500)).await;

    mac.copy
        .send("ssh root@10.0.0.5 -p 2222".into())
        .await
        .unwrap();
    assert_eq!(
        next(&mut linux).await.as_deref(),
        Some("ssh root@10.0.0.5 -p 2222")
    );

    linux.copy.send("Unicode: çğıöşü 🙂".into()).await.unwrap();
    assert_eq!(next(&mut mac).await.as_deref(), Some("Unicode: çğıöşü 🙂"));

    // The sender does not get its own message back; a device with another key gets nothing.
    let quiet = Duration::from_millis(500);
    assert!(timeout(quiet, mac.paste.recv()).await.is_err());
    assert!(timeout(quiet, stranger.paste.recv()).await.is_err());
}

#[tokio::test]
async fn invalid_channel_is_rejected() {
    let server = relay_address().await;
    let result = tokio_tungstenite::connect_async(format!("{server}/ws?channel=guess")).await;
    assert!(result.is_err());
}
