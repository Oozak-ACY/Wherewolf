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

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
    /// Host only: replace the Settings.
    UpdateSettings { settings: Settings },
    /// Host only: deal the Roles and start the Game.
    Start,
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
    /// Only the Host can do this.
    NotHost,
    /// A Role count or timer is out of bounds.
    InvalidSettings,
    /// A Game takes at least [`MIN_PLAYERS`] Players.
    NotEnoughPlayers,
    /// The Settings must hold exactly one Role per Player.
    RoleCountMismatch,
    /// The Game has started: the Settings and the seats are frozen.
    GameStarted,
}

/// Where the engine draws its chance from (dealing the Roles, and later tie
/// breaks), injected so a test can replay any Game.
pub trait Randomness: Send {
    /// A number in `0..n`, with `n > 0`.
    fn below(&mut self, n: usize) -> usize;
}

/// The secret card dealt to a Player when the Game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Role {
    Werewolf,
    Seer,
    Witch,
    Hunter,
    Villager,
}

impl Role {
    pub fn camp(self) -> Camp {
        match self {
            Role::Werewolf => Camp::Werewolves,
            Role::Seer | Role::Witch | Role::Hunter | Role::Villager => Camp::Village,
        }
    }
}

/// The side a Player wins or loses with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Camp {
    Village,
    Werewolves,
}

/// A Player's own Role, as only they see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleCard {
    pub role: Role,
    pub camp: Camp,
    /// For a Werewolf, the other Werewolves. Empty for everyone else.
    pub fellow_werewolves: Vec<PlayerId>,
}

/// A Player as everyone in the Lobby sees them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PlayerSummary {
    pub id: PlayerId,
    pub name: String,
    pub connected: bool,
}

/// How many of each Role are in play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RoleCounts {
    pub werewolf: u8,
    pub seer: u8,
    pub witch: u8,
    pub hunter: u8,
    pub villager: u8,
}

impl RoleCounts {
    /// The Narrator's balanced set for this many Players. Below 5 Players it
    /// suggests the 5-Player set, since no Game can start yet.
    pub fn suggested(players: usize) -> Self {
        let (werewolf, hunter) = match players {
            0..=5 => (1, 0),
            6..=8 => (2, 1),
            _ => (3, 1),
        };
        let special = werewolf + 1 + 1 + hunter;
        Self {
            werewolf,
            seer: 1,
            witch: 1,
            hunter,
            villager: (players.max(5) as u8).saturating_sub(special),
        }
    }
}

impl RoleCounts {
    /// One entry per Role card, in a fixed order.
    fn cards(&self) -> Vec<Role> {
        [
            (Role::Werewolf, self.werewolf),
            (Role::Seer, self.seer),
            (Role::Witch, self.witch),
            (Role::Hunter, self.hunter),
            (Role::Villager, self.villager),
        ]
        .into_iter()
        .flat_map(|(role, count)| std::iter::repeat_n(role, count as usize))
        .collect()
    }

    pub fn total(&self) -> usize {
        [
            self.werewolf,
            self.seer,
            self.witch,
            self.hunter,
            self.villager,
        ]
        .iter()
        .map(|&n| n as usize)
        .sum()
    }
}

/// How long each timed moment lasts, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Timers {
    pub discussion: u32,
    pub election: u32,
    pub vote: u32,
    pub seer: u32,
    pub werewolves: u32,
    pub witch: u32,
    pub hunter: u32,
    pub mayor_successor: u32,
    pub mayor_tie_break: u32,
}

impl Timers {
    fn all(&self) -> [u32; 9] {
        [
            self.discussion,
            self.election,
            self.vote,
            self.seer,
            self.werewolves,
            self.witch,
            self.hunter,
            self.mayor_successor,
            self.mayor_tie_break,
        ]
    }
}

impl Default for Timers {
    fn default() -> Self {
        Self {
            discussion: 300,
            election: 60,
            vote: 60,
            seer: 30,
            werewolves: 60,
            witch: 30,
            hunter: 30,
            mayor_successor: 30,
            mayor_tie_break: 30,
        }
    }
}

/// Every timer lasts between these many seconds.
pub const TIMER_BOUNDS: std::ops::RangeInclusive<u32> = 10..=1800;

/// The options the Host chooses before starting a Game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    pub roles: RoleCounts,
    pub timers: Timers,
}

/// Everything one Player is allowed to know right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlayerView {
    pub code: LobbyCode,
    pub you: PlayerId,
    pub host: PlayerId,
    /// In the order they joined.
    pub players: Vec<PlayerSummary>,
    pub settings: Settings,
    /// The Narrator's Role set for the current Player count. While the
    /// Settings use it, they follow the Player count.
    pub suggested_roles: RoleCounts,
    /// Why the Host could not start the Game right now, if they could not.
    pub start_blocked_by: Option<Rejection>,
    /// This Player's own Role, once the Game has started.
    pub role: Option<RoleCard>,
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
pub const MIN_PLAYERS: usize = 5;
pub const MAX_PLAYERS: usize = 12;

/// Long enough for a first name, short enough to fit under a video tile.
pub const MAX_NAME_CHARS: usize = 20;

#[derive(Debug)]
struct Seat {
    token: SeatToken,
    id: PlayerId,
    name: String,
    connected: bool,
    /// Dealt when the Game starts.
    role: Option<Role>,
}

pub struct Engine {
    code: LobbyCode,
    /// In the order they joined.
    seats: Vec<Seat>,
    next_id: u32,
    /// `None` while the Role set follows the Narrator's suggestion.
    custom_roles: Option<RoleCounts>,
    timers: Timers,
    started: bool,
    randomness: Box<dyn Randomness>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("code", &self.code)
            .field("seats", &self.seats)
            .field("started", &self.started)
            .finish_non_exhaustive()
    }
}

impl Engine {
    pub fn new(code: LobbyCode, randomness: impl Randomness + 'static) -> Self {
        Self {
            code,
            seats: Vec::new(),
            next_id: 1,
            custom_roles: None,
            timers: Timers::default(),
            started: false,
            randomness: Box::new(randomness),
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
                self.require_lobby()?;
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
                    role: None,
                });
            }
            Command::Leave => {
                let index = self.seat_index(seat).ok_or(Rejection::NotSeated)?;
                // A Player who walks out mid-Game keeps their seat and Role.
                self.require_lobby()?;
                self.seats.remove(index);
            }
            Command::Disconnect => {
                self.seat_mut(seat).ok_or(Rejection::NotSeated)?.connected = false;
            }
            Command::UpdateSettings { settings } => {
                self.require_host(seat)?;
                self.require_lobby()?;
                let timers_valid = settings
                    .timers
                    .all()
                    .iter()
                    .all(|t| TIMER_BOUNDS.contains(t));
                if settings.roles.total() > MAX_PLAYERS || !timers_valid {
                    return Err(Rejection::InvalidSettings);
                }
                self.custom_roles = Some(settings.roles).filter(|&r| r != self.suggested_roles());
                self.timers = settings.timers;
            }
            Command::Start => {
                self.require_host(seat)?;
                if let Some(blocker) = self.start_blocker() {
                    return Err(blocker);
                }
                self.deal();
                self.started = true;
            }
        }
        Ok(self.outputs())
    }

    /// Nobody is seated any more: the Lobby can be forgotten.
    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }

    fn require_host(&self, token: &SeatToken) -> Result<(), Rejection> {
        match self.seats.first() {
            Some(host) if &host.token == token => Ok(()),
            _ if self.seat_index(token).is_some() => Err(Rejection::NotHost),
            _ => Err(Rejection::NotSeated),
        }
    }

    fn require_lobby(&self) -> Result<(), Rejection> {
        if self.started {
            Err(Rejection::GameStarted)
        } else {
            Ok(())
        }
    }

    /// Shuffles the Settings' Role cards and hands one to each seat.
    fn deal(&mut self) {
        let mut cards = self.settings().roles.cards();
        for i in (1..cards.len()).rev() {
            let j = self.randomness.below(i + 1);
            cards.swap(i, j);
        }
        for (seat, role) in self.seats.iter_mut().zip(cards) {
            seat.role = Some(role);
        }
    }

    fn role_card(&self, seat: &Seat) -> Option<RoleCard> {
        let role = seat.role?;
        let fellow_werewolves = if role == Role::Werewolf {
            self.seats
                .iter()
                .filter(|s| s.id != seat.id && s.role == Some(Role::Werewolf))
                .map(|s| s.id)
                .collect()
        } else {
            Vec::new()
        };
        Some(RoleCard {
            role,
            camp: role.camp(),
            fellow_werewolves,
        })
    }

    fn start_blocker(&self) -> Option<Rejection> {
        if self.started {
            Some(Rejection::GameStarted)
        } else if self.seats.len() < MIN_PLAYERS {
            Some(Rejection::NotEnoughPlayers)
        } else if self.settings().roles.total() != self.seats.len() {
            Some(Rejection::RoleCountMismatch)
        } else {
            None
        }
    }

    fn suggested_roles(&self) -> RoleCounts {
        RoleCounts::suggested(self.seats.len())
    }

    fn settings(&self) -> Settings {
        Settings {
            roles: self.custom_roles.unwrap_or_else(|| self.suggested_roles()),
            timers: self.timers,
        }
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
                    settings: self.settings(),
                    suggested_roles: self.suggested_roles(),
                    start_blocked_by: self.start_blocker(),
                    role: self.role_card(s),
                };
                (s.token.clone(), view)
            })
            .collect();
        Outputs { views }
    }
}
