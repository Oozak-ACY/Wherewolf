//! The messages exchanged over each Player's WebSocket.
//!
//! These definitions are the single source of truth: the client's TypeScript
//! types are generated from them by `cargo test` (see `.cargo/config.toml`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use wherewolf_engine::{PlayerView, Rejection, Settings};

/// Client → server.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ClientMessage {
    /// Take a seat, or reclaim the one this browser's token already holds.
    Join { name: String, seat_token: String },
    /// Give up the seat for good.
    Leave,
    /// Host only: replace the Settings.
    UpdateSettings { settings: Settings },
    /// Host only: deal the Roles and start the Game.
    Start,
}

/// Server → client.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ServerMessage {
    /// Everything this Player may know, sent after every change.
    View { view: PlayerView },
    /// Sent after each accepted Join: how to enter the Lobby's video call.
    Call { ticket: CallTicket },
    /// The last command was refused and changed nothing.
    Rejected { reason: Rejection },
    /// No Lobby has this code (mistyped, or the server restarted).
    LobbyNotFound,
}

/// Response to `POST /api/lobbies`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CreatedLobby {
    pub code: String,
}

/// What a Player needs to enter the Lobby's video call.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CallTicket {
    /// The LiveKit server to connect to.
    pub url: String,
    /// LiveKit access token for this Player only.
    pub token: String,
}
