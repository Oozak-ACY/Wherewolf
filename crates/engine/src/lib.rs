//! The Narrator engine: every Wherewolf rule, as a pure state machine.
//!
//! No network, no clock, no media. The game server feeds it [`Command`]s on
//! behalf of a seat and relays the per-Player [`PlayerView`]s it returns.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The short code that identifies a Lobby, shared with friends in the link.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LobbyCode(String);

impl LobbyCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The secret random token a browser keeps to hold (and reclaim) its seat.
/// Never shown to other Players.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeatToken(String);

impl SeatToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

/// The public identity of a Player within a Lobby.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlayerId(u32);

/// What a seat asks the engine to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Take a seat in the Lobby, or reclaim the seat this token already holds.
    Join { name: String },
    /// Give up the seat for good. If it was the Host's, the Player who joined
    /// next becomes Host.
    Leave,
    /// The seat's connection dropped (phone locked, network lost). The seat is
    /// kept, so joining again with the same token reclaims it.
    Disconnect,
}

/// Why a command was refused. The engine's state is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Rejection {
    /// The Lobby already seats the maximum number of Players.
    LobbyFull,
    /// This command needs a seat, and the token holds none.
    NotSeated,
    /// The display name is blank or longer than [`MAX_NAME_CHARS`].
    InvalidName,
}

/// A Player as everyone in the Lobby sees them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PlayerSummary {
    pub id: PlayerId,
    pub name: String,
    pub connected: bool,
}

/// Everything one Player is allowed to know right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PlayerView {
    pub code: LobbyCode,
    pub you: PlayerId,
    pub host: PlayerId,
    /// In the order they joined.
    pub players: Vec<PlayerSummary>,
}

/// What the engine produces after an accepted command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outputs {
    views: Vec<(SeatToken, PlayerView)>,
}

impl Outputs {
    /// The fresh view for the Player holding this seat, if they are seated.
    pub fn view_for(&self, seat: &SeatToken) -> Option<&PlayerView> {
        self.views.iter().find(|(s, _)| s == seat).map(|(_, v)| v)
    }

    /// Every seated Player's fresh view.
    pub fn views(&self) -> impl Iterator<Item = (&SeatToken, &PlayerView)> {
        self.views.iter().map(|(s, v)| (s, v))
    }
}

/// A Game takes 5 to 12 Players.
pub const MAX_PLAYERS: usize = 12;

/// Long enough for a first name, short enough to fit under a video tile.
pub const MAX_NAME_CHARS: usize = 20;

#[derive(Debug)]
struct Seat {
    token: SeatToken,
    id: PlayerId,
    name: String,
    connected: bool,
}

#[derive(Debug)]
pub struct Engine {
    code: LobbyCode,
    /// In the order they joined.
    seats: Vec<Seat>,
    next_id: u32,
}

impl Engine {
    pub fn new(code: LobbyCode) -> Self {
        Self {
            code,
            seats: Vec::new(),
            next_id: 1,
        }
    }

    pub fn handle(&mut self, seat: &SeatToken, command: Command) -> Result<Outputs, Rejection> {
        match command {
            Command::Join { name } => {
                let name = name.trim().to_string();
                if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
                    return Err(Rejection::InvalidName);
                }
                if let Some(reclaimed) = self.seat_mut(seat) {
                    reclaimed.name = name;
                    reclaimed.connected = true;
                    return Ok(self.outputs());
                }
                if self.seats.len() >= MAX_PLAYERS {
                    return Err(Rejection::LobbyFull);
                }
                let id = PlayerId(self.next_id);
                self.next_id += 1;
                self.seats.push(Seat {
                    token: seat.clone(),
                    id,
                    name,
                    connected: true,
                });
            }
            Command::Leave => {
                let index = self.seat_index(seat).ok_or(Rejection::NotSeated)?;
                self.seats.remove(index);
            }
            Command::Disconnect => {
                self.seat_mut(seat).ok_or(Rejection::NotSeated)?.connected = false;
            }
        }
        Ok(self.outputs())
    }

    /// Nobody is seated any more: the Lobby can be forgotten.
    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }

    fn seat_mut(&mut self, token: &SeatToken) -> Option<&mut Seat> {
        self.seats.iter_mut().find(|s| &s.token == token)
    }

    fn seat_index(&self, token: &SeatToken) -> Option<usize> {
        self.seats.iter().position(|s| &s.token == token)
    }

    fn outputs(&self) -> Outputs {
        let players: Vec<PlayerSummary> = self
            .seats
            .iter()
            .map(|s| PlayerSummary {
                id: s.id,
                name: s.name.clone(),
                connected: s.connected,
            })
            .collect();
        // The Host is whoever has been seated the longest.
        let Some(host) = self.seats.first().map(|s| s.id) else {
            return Outputs { views: Vec::new() };
        };
        let views = self
            .seats
            .iter()
            .map(|s| {
                let view = PlayerView {
                    code: self.code.clone(),
                    you: s.id,
                    host,
                    players: players.clone(),
                };
                (s.token.clone(), view)
            })
            .collect();
        Outputs { views }
    }
}
