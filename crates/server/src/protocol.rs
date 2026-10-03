//! The messages exchanged over each Player's WebSocket.
//!
//! These definitions are the single source of truth: the client's TypeScript
//! types are generated from them by `cargo test` (see `.cargo/config.toml`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use wherewolf_engine::{PlayerId, PlayerView, Rejection, Settings};

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
    /// Seer's Turn: see the Role of another living Player, once.
    Inspect { player: PlayerId },
    /// Werewolves' Turn: choose (or change) the Victim.
    PickVictim { victim: PlayerId },
    /// Witch's Turn: save the Victim with the healing potion, once a Game.
    Heal,
    /// Witch's Turn: eliminate a living Player at dawn with the poison
    /// potion, once a Game.
    Poison { player: PlayerId },
    /// Witch's Turn: use no more potions tonight.
    Pass,
    /// The Election: vote for a living Player, yourself included, to be
    /// Mayor. Can be changed until the Election ends.
    Elect { candidate: PlayerId },
    /// The Vote: designate a Player, or abstain with `null`. Can be changed
    /// until the Vote ends.
    Vote { designated: Option<PlayerId> },
    /// The Mayor, on a tied Vote: choose which of the tied Players is
    /// eliminated.
    BreakTie { player: PlayerId },
    /// The eliminated Hunter's shot: eliminate a living Player, once.
    Shoot { player: PlayerId },
    /// The eliminated Mayor: name a living Player to succeed them.
    NameSuccessor { player: PlayerId },
    /// Host only, once the Game is over: everyone goes back to the Lobby.
    PlayAgain,
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
    View {
        view: Box<PlayerView>,
        /// Time left in the Game's current moment when this was sent, if it is timed.
        ends_in_ms: Option<u32>,
    },
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
