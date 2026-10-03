//! The Narrator engine: every Wherewolf rule, as a pure state machine.
//!
//! No network, no clock, no media. The game server feeds it [`Command`]s on
//! behalf of a seat and relays the per-Player [`PlayerView`]s it returns.

use std::collections::VecDeque;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
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
    /// After the start, a newcomer is a Spectator until the next Game.
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
    /// Seer's Turn: see the Role of another living Player, once.
    Inspect { player: PlayerId },
    /// Werewolves' Turn: choose (or change) the Victim.
    PickVictim { victim: PlayerId },
    /// Witch's Turn: save the Victim with the healing potion, once a Game.
    Heal,
    /// Witch's Turn: eliminate a living Player at dawn with the poison
    /// potion, once a Game.
    Poison { player: PlayerId },
    /// The Election: vote for a living Player, yourself included, to be
    /// Mayor. Can be changed until the Election ends.
    Elect { candidate: PlayerId },
    /// The Vote: designate a Player to eliminate, or abstain with `None`.
    /// Can be changed until the Vote ends.
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
    /// Someone else's Turn: only the Seer inspects, only the Werewolves pick
    /// a Victim, only the Witch uses potions.
    NotYourTurn,
    /// The Witch has already used this potion in this Game.
    PotionUsed,
    /// There is no Victim to heal tonight.
    NoVictim,
    /// A Spectator (an eliminated Player) can no longer act or vote.
    Spectating,
    /// The chosen Player has been eliminated, or is not in this Game.
    NotInPlay,
    /// A Player cannot choose themselves for this.
    Yourself,
    /// The Mayor must choose among the Players tied by the Vote.
    NotTied,
}

/// Where the engine draws its chance from (dealing the Roles, and drawing
/// the Mayor by lot), injected so a test can replay any Game.
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
    /// For the Witch, the potions she has left. `None` for everyone else.
    pub potions: Option<Potions>,
}

/// Which of the Witch's potions are still unused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Potions {
    pub healing: bool,
    pub poison: bool,
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
    /// Whether this Player is a Spectator. An eliminated Player still owed an
    /// action (like the Hunter's shot) is not one until it is resolved.
    pub spectating: bool,
    /// What is happening in the Game right now. `None` in the Lobby.
    pub moment: Option<Moment>,
    /// The Mayor, once elected.
    pub mayor: Option<PlayerId>,
    /// The visibility plan, from this Player's side: who they may see and
    /// hear right now.
    pub receives: Vec<PlayerId>,
    /// Who may see and hear this Player right now: the only ones allowed to
    /// receive their camera and microphone.
    pub audience: Vec<PlayerId>,
}

impl PlayerView {
    /// False while this Player may see and hear nobody at all.
    pub fn receives_anyone(&self) -> bool {
        !self.receives.is_empty()
    }
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
    /// Night: the Seer inspects another living Player's Role.
    SeersTurn {
        /// What the Seer learned this Turn. Only the Seer and the Spectators
        /// see it.
        inspection: Option<Inspection>,
    },
    /// Night: the Werewolves choose their Victim.
    WerewolvesTurn {
        /// Each Werewolf's current pick, live. Only the Werewolves and the
        /// Spectators see them.
        picks: Option<Vec<Pick>>,
    },
    /// Night: the Witch learns the Victim and may use her potions.
    WitchsTurn {
        /// What the Witch knows and did this Turn. Only the Witch and the
        /// Spectators see it.
        witch: Option<WitchSight>,
    },
    /// The village wakes up and learns who died in the Night, in seat order,
    /// never why.
    Dawn { deaths: Vec<PlayerId> },
    /// Day: the living debate.
    Discussion,
    /// The first Day: the living elect the Mayor. Ballots stay secret until
    /// everyone has voted or the timer ends.
    Election {
        /// Who has already voted, but not for whom.
        voted: Vec<PlayerId>,
        /// This Player's own vote so far.
        your_ballot: Option<Ballot>,
    },
    /// The Election revealed, and who it made Mayor.
    ElectionResult {
        /// Who voted for whom, in the order they first voted.
        ballots: Vec<Ballot>,
        mayor: PlayerId,
        /// Whether the Mayor was drawn at random, among the tied (or among
        /// all the living if nobody voted).
        by_lot: bool,
    },
    /// Day: the living vote to eliminate someone. Ballots stay secret until
    /// everyone has voted or the timer ends.
    Vote {
        /// Who has already voted (or abstained), but not for whom.
        voted: Vec<PlayerId>,
        /// This Player's own vote so far.
        your_ballot: Option<Ballot>,
    },
    /// Day: the Vote revealed and tied. The Mayor chooses which of the tied
    /// Players is eliminated; if they don't in time, nobody is.
    TieBreak {
        /// Who voted for whom, in the order they first voted.
        ballots: Vec<Ballot>,
        /// The Players tied with the most votes.
        tied: Vec<PlayerId>,
    },
    /// Day: the Vote revealed, and who it eliminated.
    VoteResult {
        /// Who voted for whom, in the order they first voted.
        ballots: Vec<Ballot>,
        eliminated: Option<PlayerId>,
    },
    /// The eliminated Hunter picks a living Player to shoot.
    HuntersShot { hunter: PlayerId },
    /// Who the Hunter shot, if he shot anyone before the time ran out.
    ShotResult {
        hunter: PlayerId,
        shot: Option<PlayerId>,
    },
    /// The eliminated Mayor names a living Player to succeed them.
    Succession { mayor: PlayerId },
    /// The new Mayor.
    SuccessionResult {
        successor: PlayerId,
        /// Whether they were drawn at random, the eliminated Mayor having
        /// named nobody in time.
        by_lot: bool,
    },
    /// The Game is over. Every Role is revealed.
    Victory { winner: Camp },
}

/// The Role the Seer saw on a Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Inspection {
    pub player: PlayerId,
    pub role: Role,
}

/// The Witch's Turn, as the Witch sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WitchSight {
    /// The Werewolves' Victim tonight, if they chose one.
    pub victim: Option<PlayerId>,
    /// Whether she saved the Victim tonight.
    pub healed: bool,
    /// Who she poisoned tonight.
    pub poisoned: Option<PlayerId>,
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

/// A Game has at most one Witch: her potions are the Game's, not hers.
pub const MAX_WITCHES: u8 = 1;

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
    /// The Witch's potions left, for the whole Game.
    potions: Potions,
    /// The death triggers still to resolve, in the order the deaths happened.
    triggers: VecDeque<Trigger>,
    /// `None` until the Election, on the first Day.
    mayor: Option<PlayerId>,
}

/// What a Player's Elimination sets off, resolved one at a time before the
/// Game moves on (and before any victory is checked).
#[derive(Debug, Clone, Copy)]
enum Trigger {
    HuntersShot { hunter: PlayerId },
    Succession { mayor: PlayerId },
}

impl Trigger {
    /// The death triggers `dead` sets off, in the order they are resolved:
    /// a Hunter who is also Mayor shoots first, then names a successor.
    fn set_off_by(dead: PlayerId, role: Option<Role>, was_mayor: bool) -> Vec<Self> {
        let shot = (role == Some(Role::Hunter)).then_some(Trigger::HuntersShot { hunter: dead });
        let succession = was_mayor.then_some(Trigger::Succession { mayor: dead });
        shot.into_iter().chain(succession).collect()
    }

    /// The eliminated Player this trigger belongs to.
    fn owner(self) -> PlayerId {
        match self {
            Trigger::HuntersShot { hunter } => hunter,
            Trigger::Succession { mayor } => mayor,
        }
    }
}

/// Where the Game goes once every death trigger is resolved.
#[derive(Debug, Clone, Copy)]
enum Resume {
    /// The Night's deaths are announced: the Day begins.
    Day,
    /// The Vote's result is announced: the Night falls.
    Night,
}

/// Where the Game stands.
#[derive(Debug)]
enum MomentState {
    SeersTurn {
        inspection: Option<Inspection>,
    },
    /// In the order they were made.
    WerewolvesTurn {
        picks: Vec<Pick>,
    },
    WitchsTurn {
        sight: WitchSight,
    },
    Dawn {
        deaths: Vec<PlayerId>,
    },
    Discussion,
    /// In the order they were cast.
    Election {
        ballots: Vec<Ballot>,
    },
    ElectionResult {
        ballots: Vec<Ballot>,
        mayor: PlayerId,
        by_lot: bool,
    },
    /// In the order they were cast.
    Vote {
        ballots: Vec<Ballot>,
    },
    TieBreak {
        ballots: Vec<Ballot>,
        tied: Vec<PlayerId>,
    },
    VoteResult {
        ballots: Vec<Ballot>,
        eliminated: Option<PlayerId>,
    },
    /// `then`: where the Game goes once every death trigger is resolved.
    HuntersShot {
        hunter: PlayerId,
        then: Resume,
    },
    ShotResult {
        hunter: PlayerId,
        shot: Option<PlayerId>,
        then: Resume,
    },
    Succession {
        mayor: PlayerId,
        then: Resume,
    },
    SuccessionResult {
        successor: PlayerId,
        by_lot: bool,
        then: Resume,
    },
    Victory {
        winner: Camp,
    },
}

/// How long the Narrator's announcements stay on screen, in seconds.
pub const ANNOUNCEMENT_SECONDS: u32 = 10;

/// The Players designated most often, tied, in the order they were first
/// designated. Empty if nobody was.
fn top_designated(designated: impl Iterator<Item = PlayerId>) -> Vec<PlayerId> {
    let mut counts: Vec<(PlayerId, usize)> = Vec::new();
    for id in designated {
        match counts.iter_mut().find(|(c, _)| *c == id) {
            Some((_, n)) => *n += 1,
            None => counts.push((id, 1)),
        }
    }
    let top = counts.iter().map(|&(_, n)| n).max().unwrap_or(0);
    counts
        .into_iter()
        .filter(|&(_, n)| n == top)
        .map(|(id, _)| id)
        .collect()
}

/// The single Player designated most often, unless several are tied (or
/// nobody was).
fn single_most_designated(designated: impl Iterator<Item = PlayerId>) -> Option<PlayerId> {
    match top_designated(designated)[..] {
        [id] => Some(id),
        _ => None,
    }
}

/// Records `voter`'s ballot, replacing their earlier one.
fn cast(ballots: &mut Vec<Ballot>, voter: PlayerId, designated: Option<PlayerId>) {
    match ballots.iter_mut().find(|b| b.voter == voter) {
        Some(ballot) => ballot.designated = designated,
        None => ballots.push(Ballot { voter, designated }),
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
            MomentState::SeersTurn { .. } => timers.seer,
            MomentState::WerewolvesTurn { .. } => timers.werewolves,
            MomentState::WitchsTurn { .. } => timers.witch,
            MomentState::Dawn { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::Discussion => timers.discussion,
            MomentState::Election { .. } => timers.election,
            MomentState::ElectionResult { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::Vote { .. } => timers.vote,
            MomentState::TieBreak { .. } => timers.mayor_tie_break,
            MomentState::VoteResult { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::HuntersShot { .. } => timers.hunter,
            MomentState::ShotResult { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::Succession { .. } => timers.mayor_successor,
            MomentState::SuccessionResult { .. } => ANNOUNCEMENT_SECONDS,
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
            MomentState::SeersTurn { inspection } => Moment::SeersTurn {
                // The Seer and the Spectators see it.
                inspection: (viewer.role == Some(Role::Seer) || !viewer.alive)
                    .then_some(*inspection)
                    .flatten(),
            },
            MomentState::WerewolvesTurn { picks } => Moment::WerewolvesTurn {
                // The Werewolves and the Spectators see them.
                picks: (viewer.role == Some(Role::Werewolf) || !viewer.alive)
                    .then(|| picks.clone()),
            },
            MomentState::WitchsTurn { sight } => Moment::WitchsTurn {
                // The Witch and the Spectators see it.
                witch: (viewer.role == Some(Role::Witch) || !viewer.alive).then_some(*sight),
            },
            MomentState::Dawn { deaths } => Moment::Dawn {
                deaths: deaths.clone(),
            },
            MomentState::Discussion => Moment::Discussion,
            MomentState::Election { ballots } => Moment::Election {
                voted: ballots.iter().map(|b| b.voter).collect(),
                your_ballot: ballots.iter().find(|b| b.voter == viewer.id).copied(),
            },
            MomentState::ElectionResult {
                ballots,
                mayor,
                by_lot,
            } => Moment::ElectionResult {
                ballots: ballots.clone(),
                mayor: *mayor,
                by_lot: *by_lot,
            },
            MomentState::Vote { ballots } => Moment::Vote {
                voted: ballots.iter().map(|b| b.voter).collect(),
                your_ballot: ballots.iter().find(|b| b.voter == viewer.id).copied(),
            },
            MomentState::TieBreak { ballots, tied } => Moment::TieBreak {
                ballots: ballots.clone(),
                tied: tied.clone(),
            },
            MomentState::VoteResult {
                ballots,
                eliminated,
            } => Moment::VoteResult {
                ballots: ballots.clone(),
                eliminated: *eliminated,
            },
            MomentState::HuntersShot { hunter, .. } => Moment::HuntersShot { hunter: *hunter },
            MomentState::ShotResult { hunter, shot, .. } => Moment::ShotResult {
                hunter: *hunter,
                shot: *shot,
            },
            MomentState::Succession { mayor, .. } => Moment::Succession { mayor: *mayor },
            MomentState::SuccessionResult {
                successor, by_lot, ..
            } => Moment::SuccessionResult {
                successor: *successor,
                by_lot: *by_lot,
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
                    // Someone arriving after the start watches as a Spectator.
                    alive: self.game.is_none(),
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
                if settings.roles.total() > MAX_PLAYERS
                    || settings.roles.witch > MAX_WITCHES
                    || !timers_valid
                {
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
                    state: self.nightfall(),
                    moment_id: MomentId(0),
                    potions: Potions {
                        healing: true,
                        poison: true,
                    },
                    triggers: VecDeque::new(),
                    mayor: None,
                });
            }
            Command::Inspect { player } => {
                let seer = self.require_living(seat)?;
                let is_seer = self.seat_of(seat)?.role == Some(Role::Seer);
                let Some(MomentState::SeersTurn { inspection }) = self.state() else {
                    return Err(Rejection::NotNow);
                };
                if !is_seer {
                    return Err(Rejection::NotYourTurn);
                }
                // She inspects only once a Night.
                if inspection.is_some() {
                    return Err(Rejection::NotNow);
                }
                self.require_in_play(Some(player))?;
                if player == seer {
                    return Err(Rejection::Yourself);
                }
                let role = self
                    .living()
                    .find(|s| s.id == player)
                    .and_then(|s| s.role)
                    .expect("a living Player of this Game has a Role");
                let MomentState::SeersTurn { inspection } = &mut self.game_mut().state else {
                    unreachable!("checked above")
                };
                *inspection = Some(Inspection { player, role });
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
                    self.end_werewolves_turn(Some(victim));
                }
            }
            Command::Heal => {
                self.require_witchs_turn(seat)?;
                if !self.game_mut().potions.healing {
                    return Err(Rejection::PotionUsed);
                }
                let sight = self.witch_sight_mut();
                if sight.victim.is_none() {
                    return Err(Rejection::NoVictim);
                }
                sight.healed = true;
                self.game_mut().potions.healing = false;
            }
            Command::Poison { player } => {
                self.require_witchs_turn(seat)?;
                if !self.game_mut().potions.poison {
                    return Err(Rejection::PotionUsed);
                }
                self.require_in_play(Some(player))?;
                self.witch_sight_mut().poisoned = Some(player);
                self.game_mut().potions.poison = false;
            }
            Command::Elect { candidate } => {
                let voter = self.require_living(seat)?;
                if !matches!(self.state(), Some(MomentState::Election { .. })) {
                    return Err(Rejection::NotNow);
                }
                self.require_in_play(Some(candidate))?;
                let living = self.living().count();
                let MomentState::Election { ballots } = &mut self.game_mut().state else {
                    unreachable!("checked above")
                };
                cast(ballots, voter, Some(candidate));
                if ballots.len() == living {
                    self.end_election();
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
                cast(ballots, voter, designated);
                let everyone_voted = ballots.len() == self.living().count();
                if everyone_voted {
                    self.end_vote();
                }
            }
            Command::BreakTie { player } => {
                let chooser = self.require_living(seat)?;
                let Some(MomentState::TieBreak { ballots, tied }) = self.state() else {
                    return Err(Rejection::NotNow);
                };
                if self.mayor() != Some(chooser) {
                    return Err(Rejection::NotYourTurn);
                }
                if !tied.contains(&player) {
                    return Err(Rejection::NotTied);
                }
                let ballots = ballots.clone();
                self.announce_vote(ballots, Some(player));
            }
            Command::Shoot { player } => {
                let shooter = self.seat_of(seat)?.id;
                let Some(&MomentState::HuntersShot { hunter, then }) = self.state() else {
                    return Err(Rejection::NotNow);
                };
                if shooter != hunter {
                    return Err(Rejection::NotYourTurn);
                }
                self.require_in_play(Some(player))?;
                self.eliminate(player);
                self.game_mut().go_to(MomentState::ShotResult {
                    hunter,
                    shot: Some(player),
                    then,
                });
            }
            Command::NameSuccessor { player } => {
                let namer = self.seat_of(seat)?.id;
                let Some(&MomentState::Succession { mayor, then }) = self.state() else {
                    return Err(Rejection::NotNow);
                };
                if namer != mayor {
                    return Err(Rejection::NotYourTurn);
                }
                self.require_in_play(Some(player))?;
                self.hand_over_mayor(player, false, then);
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
            MomentState::SeersTurn { .. } => self.game_mut().go_to(Self::werewolves_turn()),
            MomentState::WerewolvesTurn { .. } => {
                let victim = self.most_picked();
                self.end_werewolves_turn(victim);
            }
            MomentState::WitchsTurn { sight } => {
                let sight = *sight;
                let killed = sight.victim.filter(|_| !sight.healed);
                self.end_night(killed.into_iter().chain(sight.poisoned).collect());
            }
            MomentState::Dawn { .. } => self.resolve_triggers(Resume::Day),
            // The Mayor is elected on the first Day only.
            MomentState::Discussion if game.mayor.is_none() => {
                self.game_mut().go_to(MomentState::Election {
                    ballots: Vec::new(),
                })
            }
            MomentState::Discussion | MomentState::ElectionResult { .. } => {
                self.game_mut().go_to(MomentState::Vote {
                    ballots: Vec::new(),
                })
            }
            MomentState::Election { .. } => self.end_election(),
            MomentState::Vote { .. } => self.end_vote(),
            // The Mayor did not choose: nobody is eliminated.
            MomentState::TieBreak { ballots, .. } => {
                let ballots = ballots.clone();
                self.announce_vote(ballots, None);
            }
            MomentState::VoteResult { .. } => self.resolve_triggers(Resume::Night),
            // The time is up: the shot is lost.
            &MomentState::HuntersShot { hunter, then } => {
                self.game_mut().go_to(MomentState::ShotResult {
                    hunter,
                    shot: None,
                    then,
                })
            }
            &MomentState::ShotResult { then, .. } => self.resolve_triggers(then),
            // The Mayor named nobody in time: a living Player is drawn.
            &MomentState::Succession { then, .. } => {
                let living: Vec<PlayerId> = self.living().map(|s| s.id).collect();
                let successor = self.draw(&living);
                self.hand_over_mayor(successor, true, then);
            }
            &MomentState::SuccessionResult { then, .. } => self.resolve_triggers(then),
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

    /// The Player on this seat is the living Witch, and it is her Turn.
    fn require_witchs_turn(&self, token: &SeatToken) -> Result<(), Rejection> {
        self.require_living(token)?;
        let is_witch = self.seat_of(token)?.role == Some(Role::Witch);
        if !matches!(self.state(), Some(MomentState::WitchsTurn { .. })) {
            Err(Rejection::NotNow)
        } else if !is_witch {
            Err(Rejection::NotYourTurn)
        } else {
            Ok(())
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
        let potions = self
            .game
            .as_ref()
            .filter(|_| role == Role::Witch)
            .map(|g| g.potions);
        Some(RoleCard {
            role,
            camp: role.camp(),
            fellow_werewolves,
            potions,
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

    fn witch_sight_mut(&mut self) -> &mut WitchSight {
        match &mut self.game_mut().state {
            MomentState::WitchsTurn { sight } => sight,
            _ => unreachable!("only during the Witch's Turn"),
        }
    }

    fn state(&self) -> Option<&MomentState> {
        self.game.as_ref().map(|g| &g.state)
    }

    /// The first Turn of the Night. The Night order is the Seer, then the
    /// Werewolves; the Turn of a Role nobody living holds is skipped.
    fn nightfall(&self) -> MomentState {
        if self.living().any(|s| s.role == Some(Role::Seer)) {
            MomentState::SeersTurn { inspection: None }
        } else {
            Self::werewolves_turn()
        }
    }

    /// The Werewolves' Turn, before anyone has picked.
    fn werewolves_turn() -> MomentState {
        MomentState::WerewolvesTurn { picks: Vec::new() }
    }

    /// The Werewolves have chosen their Victim, if any. The Witch wakes up
    /// next; without a living Witch, the Night is over.
    fn end_werewolves_turn(&mut self, victim: Option<PlayerId>) {
        if self.living().any(|s| s.role == Some(Role::Witch)) {
            self.game_mut().go_to(MomentState::WitchsTurn {
                sight: WitchSight {
                    victim,
                    healed: false,
                    poisoned: None,
                },
            });
        } else {
            self.end_night(victim.into_iter().collect());
        }
    }

    /// The Night is over: `deaths` die, and the village wakes up. They are
    /// announced in seat order, so the order never tells what killed whom.
    fn end_night(&mut self, mut deaths: Vec<PlayerId>) {
        deaths.sort();
        deaths.dedup();
        for &id in &deaths {
            self.eliminate(id);
        }
        self.game_mut().go_to(MomentState::Dawn { deaths });
    }

    /// The Player most picked by the Werewolves, unless several are tied.
    fn most_picked(&self) -> Option<PlayerId> {
        single_most_designated(self.picks()?.iter().map(|p| p.victim))
    }

    /// The Election is over: it is revealed, and the most-voted Player becomes
    /// Mayor. A tie, or an Election nobody voted in, is settled at random
    /// among the tied, or among all the living.
    fn end_election(&mut self) {
        let MomentState::Election { ballots } = &self.game_mut().state else {
            unreachable!("only an Election ends")
        };
        let ballots = ballots.clone();
        let mut candidates = top_designated(ballots.iter().filter_map(|b| b.designated));
        if candidates.is_empty() {
            candidates = self.living().map(|s| s.id).collect();
        }
        let by_lot = candidates.len() > 1;
        let mayor = self.draw(&candidates);
        let game = self.game_mut();
        game.mayor = Some(mayor);
        game.go_to(MomentState::ElectionResult {
            ballots,
            mayor,
            by_lot,
        });
    }

    /// The Vote is over: it is revealed, and the most-designated Player dies.
    /// A tie goes to the living Mayor, who chooses among the tied.
    fn end_vote(&mut self) {
        let MomentState::Vote { ballots } = &self.game_mut().state else {
            unreachable!("only a Vote ends")
        };
        let ballots = ballots.clone();
        let tied = top_designated(ballots.iter().filter_map(|b| b.designated));
        let mayor = self.mayor();
        let mayor_alive = self.living().any(|s| Some(s.id) == mayor);
        match tied[..] {
            [id] => self.announce_vote(ballots, Some(id)),
            [_, _, ..] if mayor_alive => self
                .game_mut()
                .go_to(MomentState::TieBreak { ballots, tied }),
            _ => self.announce_vote(ballots, None),
        }
    }

    /// The Vote's result: `eliminated`, if anyone, dies.
    fn announce_vote(&mut self, ballots: Vec<Ballot>, eliminated: Option<PlayerId>) {
        self.game_mut().go_to(MomentState::VoteResult {
            ballots,
            eliminated,
        });
        if let Some(id) = eliminated {
            self.eliminate(id);
        }
    }

    /// Resolves the next pending death trigger. Once there is none left, the
    /// Game moves on as `then` says, unless a Camp has won.
    fn resolve_triggers(&mut self, then: Resume) {
        match self.game_mut().triggers.pop_front() {
            Some(Trigger::HuntersShot { hunter }) => self
                .game_mut()
                .go_to(MomentState::HuntersShot { hunter, then }),
            // Once a Camp has won (or nobody is left alive), a successor
            // would change nothing.
            Some(Trigger::Succession { .. }) if self.winner().is_some() => {
                self.resolve_triggers(then)
            }
            Some(Trigger::Succession { mayor }) => self
                .game_mut()
                .go_to(MomentState::Succession { mayor, then }),
            None => match then {
                Resume::Day => self.unless_won(MomentState::Discussion),
                Resume::Night => self.unless_won(self.nightfall()),
            },
        }
    }

    /// One of `players`, at random. There must be at least one.
    fn draw(&mut self, players: &[PlayerId]) -> PlayerId {
        players[self.randomness.below(players.len())]
    }

    /// The Mayor, once elected.
    fn mayor(&self) -> Option<PlayerId> {
        self.game.as_ref().and_then(|g| g.mayor)
    }

    /// `successor` becomes Mayor, and everyone is told.
    fn hand_over_mayor(&mut self, successor: PlayerId, by_lot: bool, then: Resume) {
        let game = self.game_mut();
        game.mayor = Some(successor);
        game.go_to(MomentState::SuccessionResult {
            successor,
            by_lot,
            then,
        });
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

    /// `id` dies, setting off their death triggers, if they have any.
    fn eliminate(&mut self, id: PlayerId) {
        let Some(seat) = self.seats.iter_mut().find(|s| s.id == id) else {
            return;
        };
        seat.alive = false;
        let role = seat.role;
        let game = self.game_mut();
        let triggers = Trigger::set_off_by(id, role, game.mayor == Some(id));
        game.triggers.extend(triggers);
    }

    /// Whether `seat` is a Spectator. An eliminated Player whose death
    /// trigger is still unresolved stays at the table until it is: seen and
    /// heard as the living are, and knowing only what they know.
    fn spectating(&self, seat: &Seat) -> bool {
        let Some(game) = &self.game else {
            return false;
        };
        let acting = match game.state {
            MomentState::HuntersShot { hunter, .. } => hunter == seat.id,
            MomentState::Succession { mayor, .. } => mayor == seat.id,
            _ => false,
        };
        let waiting = game.triggers.iter().any(|t| t.owner() == seat.id);
        !seat.alive && !acting && !waiting
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

    /// The visibility plan: whether `receiver` may see and hear `sender` now.
    fn may_receive(&self, receiver: &Seat, sender: &Seat) -> bool {
        if receiver.id == sender.id {
            return false;
        }
        let werewolf = |s: &Seat| s.role == Some(Role::Werewolf);
        match self.state() {
            // The Lobby and the victory screen: everyone sees everyone.
            None | Some(MomentState::Victory { .. }) => true,
            // Spectators see everyone; the living never see the Spectators.
            Some(_) if self.spectating(receiver) => true,
            Some(_) if self.spectating(sender) => false,
            // The Seer and the Witch act alone, seeing and heard by no one.
            Some(MomentState::SeersTurn { .. } | MomentState::WitchsTurn { .. }) => false,
            Some(MomentState::WerewolvesTurn { .. }) => werewolf(receiver) && werewolf(sender),
            Some(
                MomentState::Dawn { .. }
                | MomentState::Discussion
                | MomentState::Election { .. }
                | MomentState::ElectionResult { .. }
                | MomentState::Vote { .. }
                | MomentState::TieBreak { .. }
                | MomentState::VoteResult { .. }
                | MomentState::HuntersShot { .. }
                | MomentState::ShotResult { .. }
                | MomentState::Succession { .. }
                | MomentState::SuccessionResult { .. },
            ) => true,
        }
    }

    /// Everyone `receiver` may see and hear now.
    fn received_by(&self, receiver: &Seat) -> Vec<PlayerId> {
        self.seats
            .iter()
            .filter(|s| self.may_receive(receiver, s))
            .map(|s| s.id)
            .collect()
    }

    /// Everyone who may see and hear `sender` now.
    fn audience_of(&self, sender: &Seat) -> Vec<PlayerId> {
        self.seats
            .iter()
            .filter(|s| self.may_receive(s, sender))
            .map(|s| s.id)
            .collect()
    }

    /// Every seated Player, with the Roles `viewer` may know: the Roles of
    /// the eliminated, and every Role for a Spectator or once the Game is over.
    fn players_seen_by(&self, viewer: &Seat) -> Vec<PlayerSummary> {
        let sees_every_role = self
            .game
            .as_ref()
            .is_some_and(|g| g.is_over() || self.spectating(viewer));
        self.seats
            .iter()
            .map(|s| PlayerSummary {
                id: s.id,
                name: s.name.clone(),
                connected: s.connected,
                alive: s.alive,
                revealed_role: s.role.filter(|_| !s.alive || sees_every_role),
            })
            .collect()
    }

    fn outputs(&self) -> Outputs {
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
                    players: self.players_seen_by(s),
                    settings: self.settings(),
                    suggested_roles: self.suggested_roles(),
                    start_blocked_by: self.start_blocker(),
                    role: self.role_card(s),
                    spectating: self.spectating(s),
                    moment: self.game.as_ref().map(|g| g.moment(s)),
                    mayor: self.mayor(),
                    receives: self.received_by(s),
                    audience: self.audience_of(s),
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
