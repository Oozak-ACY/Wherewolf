//! Wherewolf game server. Holds every Lobby in memory and relays: each
//! Player's messages go to the Narrator engine, and each Player gets back the
//! view the engine produced for them. No game rule lives here.

mod protocol;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use wherewolf_engine::{Command, Engine, LobbyCode, Outputs, Rejection, SeatToken};

use protocol::{ClientMessage, CreatedLobby, ServerMessage};

/// No 0/O or 1/I, so the code can be read aloud and typed without mistakes.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 5;
const MAX_SEAT_TOKEN_LEN: usize = 128;

type Outbox = mpsc::UnboundedSender<ServerMessage>;

/// One Lobby: its engine, plus the live connection of each seated Player.
struct Lobby {
    engine: Engine,
    connections: HashMap<SeatToken, Connection>,
}

struct Connection {
    id: u64,
    outbox: Outbox,
}

impl Lobby {
    /// Feeds a command to the engine and relays the resulting views.
    fn apply(&mut self, seat: &SeatToken, command: Command) -> Result<(), Rejection> {
        let outputs = self.engine.handle(seat, command)?;
        self.relay(&outputs);
        Ok(())
    }

    fn relay(&self, outputs: &Outputs) {
        for (seat, view) in outputs.views() {
            if let Some(connection) = self.connections.get(seat) {
                let _ = connection
                    .outbox
                    .send(ServerMessage::View { view: view.clone() });
            }
        }
    }
}

#[derive(Clone, Default)]
struct AppState {
    lobbies: Arc<Mutex<HashMap<String, Arc<Mutex<Lobby>>>>>,
    next_connection_id: Arc<AtomicU64>,
}

impl AppState {
    fn lobby(&self, code: &str) -> Option<Arc<Mutex<Lobby>>> {
        self.lobbies.lock().unwrap().get(code).cloned()
    }

    fn forget_if_empty(&self, code: &str) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if lobbies
            .get(code)
            .is_some_and(|l| l.lock().unwrap().engine.is_empty())
        {
            lobbies.remove(code);
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/api/lobbies", post(create_lobby))
        .route("/api/lobbies/:code/ws", get(connect))
        .layer(CorsLayer::permissive())
        .with_state(AppState::default());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_lobby(State(state): State<AppState>) -> Json<CreatedLobby> {
    let mut lobbies = state.lobbies.lock().unwrap();
    let code = loop {
        let code = random_code();
        if !lobbies.contains_key(&code) {
            break code;
        }
    };
    let lobby = Lobby {
        engine: Engine::new(LobbyCode::new(code.clone())),
        connections: HashMap::new(),
    };
    lobbies.insert(code.clone(), Arc::new(Mutex::new(lobby)));
    tracing::info!(code, "lobby created");
    Json(CreatedLobby { code })
}

fn random_code() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_LEN)
        .map(|_| CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

async fn connect(
    ws: WebSocketUpgrade,
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| serve_connection(socket, code.to_uppercase(), state))
}

/// Serves one Player's WebSocket until it closes.
async fn serve_connection(socket: WebSocket, code: String, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (outbox, mut inbox) = mpsc::unbounded_channel::<ServerMessage>();

    let writer = tokio::spawn(async move {
        while let Some(message) = inbox.recv().await {
            let json = serde_json::to_string(&message).expect("server messages serialize");
            if sink.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    let Some(lobby) = state.lobby(&code) else {
        let _ = outbox.send(ServerMessage::LobbyNotFound);
        drop(outbox);
        let _ = writer.await;
        return;
    };

    let connection_id = state.next_connection_id.fetch_add(1, Ordering::Relaxed);
    let mut seat: Option<SeatToken> = None;

    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(text) = message else {
            continue;
        };
        let Ok(message) = serde_json::from_str::<ClientMessage>(&text) else {
            tracing::warn!(code, "ignoring malformed message");
            continue;
        };
        let mut lobby = lobby.lock().unwrap();
        match message {
            ClientMessage::Join { name, seat_token } => {
                if seat_token.is_empty() || seat_token.len() > MAX_SEAT_TOKEN_LEN {
                    continue;
                }
                let token = SeatToken::new(seat_token);
                // One connection holds at most one seat.
                if seat.as_ref().is_some_and(|held| held != &token) {
                    continue;
                }
                // Register first so the joining Player gets their own view. A newer
                // connection for the same seat (another tab, a reconnect) replaces
                // the old one, which stops receiving views.
                let connection = Connection {
                    id: connection_id,
                    outbox: outbox.clone(),
                };
                let replaced = lobby.connections.insert(token.clone(), connection);
                match lobby.apply(&token, Command::Join { name }) {
                    Ok(()) => seat = Some(token),
                    Err(reason) => {
                        match replaced {
                            Some(previous) => lobby.connections.insert(token, previous),
                            None => lobby.connections.remove(&token),
                        };
                        let _ = outbox.send(ServerMessage::Rejected { reason });
                    }
                }
            }
            ClientMessage::Leave => {
                let Some(token) = seat.take() else { continue };
                lobby.connections.remove(&token);
                let _ = lobby.apply(&token, Command::Leave);
                drop(lobby);
                state.forget_if_empty(&code);
                break;
            }
        }
    }

    // The socket is gone. Unless a newer connection took the seat over, the
    // Player is now disconnected but keeps their seat.
    if let Some(token) = seat {
        let mut lobby = lobby.lock().unwrap();
        let current = lobby
            .connections
            .get(&token)
            .is_some_and(|c| c.id == connection_id);
        if current {
            lobby.connections.remove(&token);
            let _ = lobby.apply(&token, Command::Disconnect);
        }
    }
    drop(outbox);
    let _ = writer.await;
}
