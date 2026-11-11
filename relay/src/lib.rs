//! panosync relay.
//!
//! Devices connect with a WebSocket to `/ws?channel=<channel id>`. A binary message sent by one
//! device is forwarded to every other device connected to the same channel. `/health` answers
//! `ok` for load balancers and uptime checks.
//!
//! The relay cannot read the messages: clients encrypt everything before sending. The channel
//! id is derived from the clients' secret key, so it cannot be guessed without that key.
//! The relay keeps no state beyond the open connections.

use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, broadcast};

/// Largest accepted message (encrypted), in bytes.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
/// Most devices that can be connected to one channel at the same time.
const MAX_DEVICES_PER_CHANNEL: usize = 16;
/// Most messages one connection may send per [`RATE_WINDOW`].
const MAX_MESSAGES_PER_WINDOW: u32 = 50;
const RATE_WINDOW: Duration = Duration::from_secs(10);
/// Messages buffered per channel for slow receivers.
const CHANNEL_BUFFER: usize = 32;

/// A forwarded message, tagged with the id of the connection that sent it.
type Broadcast = (u64, Bytes);

struct Channel {
    sender: broadcast::Sender<Broadcast>,
    devices: usize,
}

#[derive(Clone, Default)]
struct AppState {
    channels: Arc<Mutex<HashMap<String, Channel>>>,
    next_connection_id: Arc<AtomicU64>,
}

#[derive(Deserialize)]
struct ChannelQuery {
    channel: String,
}

/// The relay's HTTP routes.
pub fn app() -> Router {
    Router::new()
        .route("/ws", get(upgrade))
        .route("/health", get(|| async { "ok" }))
        .with_state(AppState::default())
}

/// Serves the relay on an already bound listener.
pub async fn serve(listener: tokio::net::TcpListener) -> std::io::Result<()> {
    axum::serve(listener, app()).await
}

/// A channel id is 64 lowercase hexadecimal characters.
fn is_valid_channel_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

async fn upgrade(
    ws: WebSocketUpgrade,
    Query(query): Query<ChannelQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !is_valid_channel_id(&query.channel) {
        return (StatusCode::BAD_REQUEST, "invalid channel").into_response();
    }
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| handle_connection(socket, query.channel, state))
}

async fn handle_connection(socket: WebSocket, channel: String, state: AppState) {
    let connection_id = state.next_connection_id.fetch_add(1, Ordering::Relaxed);
    // The first 8 characters are enough to tell channels apart in the logs.
    let short = channel[..8].to_string();

    let Some((sender, receiver)) = join(&state, &channel, &short).await else {
        return;
    };
    forward(socket, connection_id, sender, receiver, &short).await;
    leave(&state, &channel).await;
    tracing::info!(channel = short, "device disconnected");
}

/// Registers a device in its channel. Returns `None` if the channel is full.
async fn join(
    state: &AppState,
    channel: &str,
    short: &str,
) -> Option<(broadcast::Sender<Broadcast>, broadcast::Receiver<Broadcast>)> {
    let mut channels = state.channels.lock().await;
    let entry = channels
        .entry(channel.to_string())
        .or_insert_with(|| Channel {
            sender: broadcast::channel(CHANNEL_BUFFER).0,
            devices: 0,
        });
    if entry.devices >= MAX_DEVICES_PER_CHANNEL {
        tracing::warn!(channel = short, "channel is full, connection rejected");
        return None;
    }
    entry.devices += 1;
    tracing::info!(channel = short, devices = entry.devices, "device connected");
    Some((entry.sender.clone(), entry.sender.subscribe()))
}

/// Removes a device from its channel, and the channel once it is empty.
async fn leave(state: &AppState, channel: &str) {
    let mut channels = state.channels.lock().await;
    if let Some(entry) = channels.get_mut(channel) {
        entry.devices -= 1;
        if entry.devices == 0 {
            channels.remove(channel);
        }
    }
}

/// Forwards messages between one device and the rest of its channel until it disconnects.
async fn forward(
    socket: WebSocket,
    connection_id: u64,
    sender: broadcast::Sender<Broadcast>,
    mut receiver: broadcast::Receiver<Broadcast>,
    short: &str,
) {
    let (mut sink, mut stream) = socket.split();
    let mut rate = RateLimit::new();

    loop {
        tokio::select! {
            message = stream.next() => match message {
                Some(Ok(Message::Binary(data))) => {
                    if !rate.allow() {
                        tracing::warn!(channel = short, "too many messages, closing connection");
                        return;
                    }
                    let _ = sender.send((connection_id, data));
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                // Pings are answered automatically; text messages are ignored.
                Some(Ok(_)) => {}
            },
            broadcast = receiver.recv() => match broadcast {
                Ok((from, data)) if from != connection_id => {
                    if sink.send(Message::Binary(data)).await.is_err() {
                        return;
                    }
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            },
        }
    }
}

/// Fixed-window message counter for one connection.
struct RateLimit {
    window_start: Instant,
    count: u32,
}

impl RateLimit {
    fn new() -> Self {
        Self {
            window_start: Instant::now(),
            count: 0,
        }
    }

    fn allow(&mut self) -> bool {
        if self.window_start.elapsed() > RATE_WINDOW {
            self.window_start = Instant::now();
            self.count = 0;
        }
        self.count += 1;
        self.count <= MAX_MESSAGES_PER_WINDOW
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_id_validation() {
        assert!(is_valid_channel_id(&"a1".repeat(32)));
        assert!(!is_valid_channel_id("short"));
        assert!(!is_valid_channel_id(&"A1".repeat(32)));
        assert!(!is_valid_channel_id(&"../".repeat(22)));
    }

    #[test]
    fn rate_limit_allows_the_window_budget() {
        let mut rate = RateLimit::new();
        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            assert!(rate.allow());
        }
        assert!(!rate.allow());
    }
}
