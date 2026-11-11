//! Connection to the relay.
//!
//! [`run`] keeps a WebSocket connection to the relay open: it sends sealed clipboard changes,
//! hands validated incoming text to the clipboard side, pings the relay to detect dead
//! connections and reconnects with exponential backoff (1 s up to 30 s).

use crate::{crypto::Keys, protocol::Codec};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

const PING_INTERVAL: Duration = Duration::from_secs(25);
/// If nothing at all is heard from the relay for this long, the connection is considered dead.
const SILENCE_LIMIT: Duration = Duration::from_secs(70);
const MAX_BACKOFF_SECS: u64 = 30;

/// Builds the WebSocket URL for a channel, accepting the server address with or without `/ws`.
pub fn relay_url(server: &str, channel_id: &str) -> String {
    let mut base = server.trim().trim_end_matches('/').to_string();
    if !base.ends_with("/ws") {
        base.push_str("/ws");
    }
    format!("{base}?channel={channel_id}")
}

/// How a connection ended.
enum Ended {
    /// The connection dropped; reconnect.
    Disconnected,
    /// The clipboard side went away; stop.
    Shutdown,
}

/// Runs until `outgoing` is closed: sends every text received on `outgoing` to the other
/// devices, and puts text from the other devices on `incoming`.
pub async fn run(
    server: String,
    keys: Keys,
    mut outgoing: mpsc::Receiver<String>,
    incoming: mpsc::Sender<String>,
) -> Result<()> {
    let mut codec = Codec::new(keys);
    let url = relay_url(&server, codec.channel_id());
    let mut backoff_secs = 1;

    loop {
        match connect_async(&url).await {
            Ok((socket, _)) => {
                backoff_secs = 1;
                tracing::info!("connected to the relay");
                match serve(socket, &mut codec, &mut outgoing, &incoming).await {
                    Ended::Shutdown => return Ok(()),
                    Ended::Disconnected => tracing::warn!("connection lost"),
                }
            }
            Err(e) => tracing::warn!("could not connect: {e}"),
        }

        tracing::info!("retrying in {backoff_secs} s");
        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
    }
}

/// Handles one open connection until it drops or the clipboard side shuts down.
async fn serve(
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    codec: &mut Codec,
    outgoing: &mut mpsc::Receiver<String>,
    incoming: &mpsc::Sender<String>,
) -> Ended {
    let (mut sink, mut stream) = socket.split();

    // Of the changes queued while offline, only the most recent one is still relevant.
    let mut latest = None;
    while let Ok(text) = outgoing.try_recv() {
        latest = Some(text);
    }
    if let Some(text) = latest
        && !send(&mut sink, codec, text).await
    {
        return Ended::Disconnected;
    }

    let mut ping = tokio::time::interval(PING_INTERVAL);
    let mut last_heard = Instant::now();

    loop {
        tokio::select! {
            text = outgoing.recv() => {
                let Some(text) = text else { return Ended::Shutdown };
                if !send(&mut sink, codec, text).await {
                    return Ended::Disconnected;
                }
            }
            message = stream.next() => {
                last_heard = Instant::now();
                match message {
                    Some(Ok(Message::Binary(sealed))) => match codec.open(&sealed) {
                        Ok(Some(text)) => {
                            tracing::info!("received ({} bytes)", text.len());
                            if incoming.send(text).await.is_err() {
                                return Ended::Shutdown;
                            }
                        }
                        Ok(None) => {}
                        Err(e) => tracing::warn!("message rejected: {e}"),
                    },
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return Ended::Disconnected,
                    Some(Ok(_)) => {}
                }
            }
            _ = ping.tick() => {
                if last_heard.elapsed() > SILENCE_LIMIT {
                    tracing::warn!("no response from the relay for too long");
                    return Ended::Disconnected;
                }
                if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return Ended::Disconnected;
                }
            }
        }
    }
}

/// Seals and sends one text. Returns `false` if the connection is broken.
async fn send<S>(sink: &mut S, codec: &Codec, text: String) -> bool
where
    S: SinkExt<Message> + Unpin,
{
    let len = text.len();
    if sink
        .send(Message::Binary(codec.seal(text).into()))
        .await
        .is_err()
    {
        return false;
    }
    tracing::info!("sent ({len} bytes)");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_url_accepts_common_forms() {
        for server in [
            "wss://p.example.com",
            "wss://p.example.com/",
            "wss://p.example.com/ws",
            " wss://p.example.com/ws/ ",
        ] {
            assert_eq!(relay_url(server, "ab"), "wss://p.example.com/ws?channel=ab");
        }
    }
}
