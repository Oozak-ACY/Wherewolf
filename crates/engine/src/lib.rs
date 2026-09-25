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
    /// Werewolves' Turn: choose (or change) the Victim.
    PickVictim { victim: PlayerId },
    /// The Vote: designate a Player to eliminate, or abstain with `None`.
    /// Can be changed until the Vote ends.
    Vote { designated: Option<PlayerId> },
    /// Host only, once the Game is over: everyone goes back to the Lobby.
    PlayAgain,
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
    /// That cannot be done at this moment of the Game.
    NotNow,
    /// Someone else's Turn: only the Werewolves pick a Victim.
    NotYourTurn,
    /// A Spectator (an eliminated Player) can no longer act or vote.
    Spectating,
    /// The chosen Player has been eliminated, or is not in this Game.
    NotInPlay,
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
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlayerSummary {
    pub id: PlayerId,
    pub name: String,
    pub connected: bool,
    /// Always true in the Lobby.
    pub alive: bool,
    /// Revealed to everyone at Elimination.
    pub revealed_role: Option<Role>,
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
    /// What is happening in the Game right now. `None` in the Lobby.
    pub moment: Option<Moment>,
}

/// A moment of the Game, as one Player sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Moment {
    /// Night: the Werewolves choose their Victim.
    WerewolvesTurn {
        /// Each Werewolf's current pick, live. Only the Werewolves see them.
        picks: Option<Vec<Pick>>,
    },
    /// The village wakes up and learns who died in the Night.
    Dawn { deaths: Vec<PlayerId> },
    /// Day: the living debate.
    Discussion,
    /// Day: the living vote to eliminate someone. Ballots stay secret until
    /// everyone has voted or the timer ends.
    Vote {
        /// Who has already voted (or abstained), but not for whom.
        voted: Vec<PlayerId>,
        /// This Player's own vote so far.
        your_ballot: Option<Ballot>,
    },
    /// Day: the Vote revealed, and who it eliminated.
    VoteResult {
        /// Who voted for whom, in the order they first voted.
        ballots: Vec<Ballot>,
        eliminated: Option<PlayerId>,
    },
    /// The Game is over. Every Role is revealed.
    Victory { winner: Camp },
}

/// A Werewolf's current choice of Victim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Pick {
    pub werewolf: PlayerId,
    pub victim: PlayerId,
}

/// One living Player's vote. `designated` is `None` for an abstention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Ballot {
    pub voter: PlayerId,
    pub designated: Option<PlayerId>,
}

/// Identifies one moment of a Game, so a timer armed for a moment that has
/// since ended can be told apart from the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MomentId(u32);

/// The deadline of the current moment: once `seconds` have passed since the
/// moment began, the server calls [`Engine::deadline_passed`] with `moment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    pub moment: MomentId,
    pub seconds: u32,
}

/// What the engine produces after an accepted command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outputs {
    views: Vec<(SeatToken, PlayerView)>,
    timer: Option<Timer>,
}

impl Outputs {
    /// The current moment's timer, if it has one. Unchanged until the moment ends.
    pub fn timer(&self) -> Option<Timer> {
        self.timer
    }

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
    alive: bool,
}

/// A Game in progress.
#[derive(Debug)]
struct Game {
    state: MomentState,
    /// Changes every time the moment does.
    moment_id: MomentId,
}

/// Where the Game stands.
#[derive(Debug)]
enum MomentState {
    /// In the order they were made.
    WerewolvesTurn {
        picks: Vec<Pick>,
    },
    Dawn {
        deaths: Vec<PlayerId>,
    },
    Discussion,
    /// In the order they were cast.
    Vote {
        ballots: Vec<Ballot>,
    },
    VoteResult {
        ballots: Vec<Ballot>,
        eliminated: Option<PlayerId>,
    },
    Victory {
        winner: Camp,
    },
}

/// How long the Narrator's announcements stay on screen, in seconds.
pub const ANNOUNCEMENT_SECONDS: u32 = 10;

/// The Player designated most often, unless several are tied (or nobody was).
fn most_designated(designated: impl Iterator<Item = PlayerId>) -> Option<PlayerId> {
    let mut counts: Vec<(PlayerId, usize)> = Vec::new();
    for id in designated {
        match counts.iter_mut().find(|(c, _)| *c == id) {
            Some((_, n)) => *n += 1,
            None => counts.push((id, 1)),
        }
    }
    let top = counts.iter().map(|&(_, n)| n).max()?;
    match counts
        .iter()
        .filter(|&&(_, n)| n == top)
        .collect::<Vec<_>>()[..]
    {
        [&(id, _)] => Some(id),
        _ => None,
    }
}

impl Game {
    fn go_to(&mut self, state: MomentState) {
        self.state = state;
        self.moment_id = MomentId(self.moment_id.0 + 1);
    }

    /// `None` once the Game is over: nothing is timed any more.
    fn timer(&self, timers: &Timers) -> Option<Timer> {
        let seconds = match self.state {
            MomentState::WerewolvesTurn { .. } => timers.werewolves,
            MomentState::Dawn { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::Discussion => timers.discussion,
            MomentState::Vote { .. } => timers.vote,
            MomentState::VoteResult { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::Victory { .. } => return None,
        };
        Some(Timer {
            moment: self.moment_id,
            seconds,
        })
    }

    fn is_over(&self) -> bool {
        matches!(self.state, MomentState::Victory { .. })
    }

    /// This moment as `viewer` may see it.
    fn moment(&self, viewer: &Seat) -> Moment {
        match &self.state {
            MomentState::WerewolvesTurn { picks } => Moment::WerewolvesTurn {
                picks: (viewer.role == Some(Role::Werewolf)).then(|| picks.clone()),
            },
            MomentState::Dawn { deaths } => Moment::Dawn {
                deaths: deaths.clone(),
            },
            MomentState::Discussion => Moment::Discussion,
            MomentState::Vote { ballots } => Moment::Vote {
                voted: ballots.iter().map(|b| b.voter).collect(),
                your_ballot: ballots.iter().find(|b| b.voter == viewer.id).copied(),
            },
            MomentState::VoteResult {
                ballots,
                eliminated,
            } => Moment::VoteResult {
                ballots: ballots.clone(),
                eliminated: *eliminated,
            },
            MomentState::Victory { winner } => Moment::Victory { winner: *winner },
        }
    }
}

pub struct Engine {
    code: LobbyCode,
    /// In the order they joined.
    seats: Vec<Seat>,
    next_id: u32,
    /// `None` while the Role set follows the Narrator's suggestion.
    custom_roles: Option<RoleCounts>,
    timers: Timers,
    /// `None` in the Lobby.
    game: Option<Game>,
    randomness: Box<dyn Randomness>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("code", &self.code)
            .field("seats", &self.seats)
            .field("game", &self.game)
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
            game: None,
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
                    alive: true,
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
                self.game = Some(Game {
                    state: MomentState::WerewolvesTurn { picks: Vec::new() },
                    moment_id: MomentId(0),
                });
            }
            Command::PickVictim { victim } => {
                let werewolf = self.require_living(seat)?;
                let is_werewolf = self.seat_of(seat)?.role == Some(Role::Werewolf);
                if !matches!(self.state(), Some(MomentState::WerewolvesTurn { .. })) {
                    return Err(Rejection::NotNow);
                }
                if !is_werewolf {
                    return Err(Rejection::NotYourTurn);
                }
                self.require_in_play(Some(victim))?;
                let picks = self.picks_mut();
                picks.retain(|p| p.werewolf != werewolf);
                picks.push(Pick { werewolf, victim });
                if let Some(victim) = self.unanimous_victim() {
                    self.end_night(Some(victim));
                }
            }
            Command::Vote { designated } => {
                let voter = self.require_living(seat)?;
                if !matches!(self.state(), Some(MomentState::Vote { .. })) {
                    return Err(Rejection::NotNow);
                }
                self.require_in_play(designated)?;
                let MomentState::Vote { ballots } = &mut self.game_mut().state else {
                    unreachable!("checked above")
                };
                match ballots.iter_mut().find(|b| b.voter == voter) {
                    Some(ballot) => ballot.designated = designated,
                    None => ballots.push(Ballot { voter, designated }),
                }
                let everyone_voted = ballots.len() == self.living().count();
                if everyone_voted {
                    self.end_vote();
                }
            }
            Command::PlayAgain => {
                self.require_host(seat)?;
                if !self.game.as_ref().is_some_and(Game::is_over) {
                    return Err(Rejection::NotNow);
                }
                self.game = None;
                for seat in &mut self.seats {
                    seat.role = None;
                    seat.alive = true;
                }
            }
        }
        Ok(self.outputs())
    }

    /// Called by the server when the timer armed for `moment` fires. Refused
    /// if that moment has already ended.
    pub fn deadline_passed(&mut self, moment: MomentId) -> Result<Outputs, Rejection> {
        let game = self.game.as_ref().ok_or(Rejection::NotNow)?;
        if game.moment_id != moment {
            return Err(Rejection::NotNow);
        }
        match &game.state {
            MomentState::WerewolvesTurn { .. } => {
                let victim = self.most_picked();
                self.end_night(victim);
            }
            MomentState::Dawn { .. } => self.unless_won(MomentState::Discussion),
            MomentState::Discussion => self.game_mut().go_to(MomentState::Vote {
                ballots: Vec::new(),
            }),
            MomentState::Vote { .. } => self.end_vote(),
            MomentState::VoteResult { .. } => {
                self.unless_won(MomentState::WerewolvesTurn { picks: Vec::new() })
            }
            MomentState::Victory { .. } => return Err(Rejection::NotNow),
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

    /// The Player on this seat, if the Game is running and they are alive.
    fn require_living(&self, token: &SeatToken) -> Result<PlayerId, Rejection> {
        let seat = self.seat_of(token)?;
        if self.game.as_ref().is_none_or(Game::is_over) {
            Err(Rejection::NotNow)
        } else if !seat.alive {
            Err(Rejection::Spectating)
        } else {
            Ok(seat.id)
        }
    }

    /// The chosen Player, if any, is a living Player of this Game.
    fn require_in_play(&self, chosen: Option<PlayerId>) -> Result<(), Rejection> {
        match chosen {
            Some(id) if !self.living().any(|s| s.id == id) => Err(Rejection::NotInPlay),
            _ => Ok(()),
        }
    }

    fn require_lobby(&self) -> Result<(), Rejection> {
        if self.game.is_some() {
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

    /// The Victim every living Werewolf picked, if they all agree.
    fn unanimous_victim(&self) -> Option<PlayerId> {
        let picks = self.picks()?;
        let mut victims = self
            .living()
            .filter(|s| s.role == Some(Role::Werewolf))
            .map(|s| picks.iter().find(|p| p.werewolf == s.id).map(|p| p.victim));
        let first = victims.next()??;
        victims.all(|v| v == Some(first)).then_some(first)
    }

    /// The Werewolves' picks, during their Turn.
    fn picks(&self) -> Option<&Vec<Pick>> {
        match self.state()? {
            MomentState::WerewolvesTurn { picks } => Some(picks),
            _ => None,
        }
    }

    fn picks_mut(&mut self) -> &mut Vec<Pick> {
        match &mut self.game_mut().state {
            MomentState::WerewolvesTurn { picks } => picks,
            _ => unreachable!("only during the Werewolves' Turn"),
        }
    }

    fn state(&self) -> Option<&MomentState> {
        self.game.as_ref().map(|g| &g.state)
    }

    /// The Night is over: its Victim, if any, dies, and the village wakes up.
    fn end_night(&mut self, victim: Option<PlayerId>) {
        let deaths: Vec<PlayerId> = victim.into_iter().collect();
        for &id in &deaths {
            self.eliminate(id);
        }
        self.game_mut().go_to(MomentState::Dawn { deaths });
    }

    /// The Player most picked by the Werewolves, unless several are tied.
    fn most_picked(&self) -> Option<PlayerId> {
        most_designated(self.picks()?.iter().map(|p| p.victim))
    }

    /// The Vote is over: it is revealed, and the most-designated Player dies.
    fn end_vote(&mut self) {
        let game = self.game_mut();
        let MomentState::Vote { ballots } = &game.state else {
            unreachable!("only a Vote ends")
        };
        let ballots = ballots.clone();
        let eliminated = most_designated(ballots.iter().filter_map(|b| b.designated));
        game.go_to(MomentState::VoteResult {
            ballots,
            eliminated,
        });
        if let Some(id) = eliminated {
            self.eliminate(id);
        }
    }

    /// Moves on to `next`, unless a Camp has won: then the Game is over.
    fn unless_won(&mut self, next: MomentState) {
        let next = match self.winner() {
            Some(winner) => MomentState::Victory { winner },
            None => next,
        };
        self.game_mut().go_to(next);
    }

    /// The Village wins once every Werewolf is dead, which takes precedence;
    /// the Werewolves win once they are at least as many as the rest.
    fn winner(&self) -> Option<Camp> {
        let werewolves = self
            .living()
            .filter(|s| s.role.map(Role::camp) == Some(Camp::Werewolves))
            .count();
        let others = self.living().count() - werewolves;
        if werewolves == 0 {
            Some(Camp::Village)
        } else if werewolves >= others {
            Some(Camp::Werewolves)
        } else {
            None
        }
    }

    fn living(&self) -> impl Iterator<Item = &Seat> {
        self.seats.iter().filter(|s| s.alive)
    }

    fn eliminate(&mut self, id: PlayerId) {
        if let Some(seat) = self.seats.iter_mut().find(|s| s.id == id) {
            seat.alive = false;
        }
    }

    fn game_mut(&mut self) -> &mut Game {
        self.game.as_mut().expect("the Game is running")
    }

    fn start_blocker(&self) -> Option<Rejection> {
        if self.game.is_some() {
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

    fn seat_of(&self, token: &SeatToken) -> Result<&Seat, Rejection> {
        self.seats
            .iter()
            .find(|s| &s.token == token)
            .ok_or(Rejection::NotSeated)
    }

    fn seat_index(&self, token: &SeatToken) -> Option<usize> {
        self.seats.iter().position(|s| &s.token == token)
    }

    fn outputs(&self) -> Outputs {
        let game_over = self.game.as_ref().is_some_and(Game::is_over);
        let players: Vec<PlayerSummary> = self
            .seats
            .iter()
            .map(|s| PlayerSummary {
                id: s.id,
                name: s.name.clone(),
                connected: s.connected,
                alive: s.alive,
                revealed_role: s.role.filter(|_| !s.alive || game_over),
            })
            .collect();
        // The Host is whoever has been seated the longest.
        let Some(host) = self.seats.first().map(|s| s.id) else {
            return Outputs {
                views: Vec::new(),
                timer: None,
            };
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
                    moment: self.game.as_ref().map(|g| g.moment(s)),
                };
                (s.token.clone(), view)
            })
            .collect();
        Outputs {
            views,
            timer: self.game.as_ref().and_then(|g| g.timer(&self.timers)),
        }
    }
}
