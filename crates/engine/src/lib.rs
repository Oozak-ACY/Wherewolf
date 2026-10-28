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
    /// kept, so joining again with the same token reclaims it; in the Lobby,
    /// only until the Grace period runs out.
    Disconnect,
    /// Host only: replace the Settings.
    UpdateSettings { settings: Settings },
    /// Host only: deal the Roles and start the Game, on the Role reveal.
    Start,
    /// The Role reveal: "I've seen my Role". Confirming again changes
    /// nothing.
    ConfirmRole,
    /// Cupid's Turn: name the two Lovers, two different living Players,
    /// Cupid included if they like. Ends the Turn.
    ChooseLovers { lovers: Vec<PlayerId> },
    /// The Lovers' Turn: "I've seen who my partner is". Confirming again
    /// changes nothing.
    ConfirmPartner,
    /// The Thief's Turn, on the second Night: rob a living Player other than
    /// themself. The Theft takes effect at dawn. Ends the Turn.
    Rob { player: PlayerId },
    /// Seer's Turn: see the Role of another living Player, once.
    Inspect { player: PlayerId },
    /// Werewolves' Turn: choose (or change) the Victim.
    PickVictim { victim: PlayerId },
    /// Witch's Turn: save a Player dying tonight with the healing potion,
    /// once a Game.
    Heal { player: PlayerId },
    /// Witch's Turn: eliminate a living Player at dawn with the poison
    /// potion, once a Game.
    Poison { player: PlayerId },
    /// Do nothing in this Moment, as letting its time run out would: the
    /// Thief robs nobody, the Seer inspects nobody, a Werewolf picks nobody
    /// (and may still change their mind), the Witch uses no more potions, the
    /// eliminated Hunter shoots nobody, the Mayor on a tie eliminates nobody.
    /// With Hidden Roles off, a Night Turn then ends as soon as its action is
    /// complete; with it on, it runs its full time. Not when the eliminated
    /// Mayor names a successor.
    Pass,
    /// The Election: vote for a living Player, yourself included, to be
    /// Mayor. Can be changed until the Election ends.
    Elect { candidate: PlayerId },
    /// The Vote: designate a Player to eliminate, or abstain with `None`.
    /// Can be changed until the Vote ends.
    Vote { designated: Option<PlayerId> },
    /// The Day's debate: end it now, as its timer would. Only the living
    /// Mayor.
    EndDebate,
    /// The Mayor, on a tied Vote: choose which of the tied Players is
    /// eliminated.
    BreakTie { player: PlayerId },
    /// The eliminated Hunter's shot: eliminate a living Player, once.
    Shoot { player: PlayerId },
    /// The eliminated Mayor: name a living Player to succeed them.
    NameSuccessor { player: PlayerId },
    /// Host only, once the Game is over: everyone goes back to the Lobby.
    PlayAgain,
    /// Host only (their phone is the Table device): the Narrator has said
    /// every line announcing `moment`, which now opens. Refused unless
    /// `moment` is the one being announced.
    Announced { moment: MomentId },
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
    /// The Game cannot start while a seated Player is Disconnected.
    PlayersDisconnected,
    /// Someone else's Turn: only Cupid names the Lovers, only the Lovers
    /// confirm their bond, only the Thief robs, only the Seer inspects, only
    /// the Werewolves pick a Victim, only the Witch uses potions.
    NotYourTurn,
    /// Cupid must name exactly two different Players.
    NotTwoPlayers,
    /// The Witch has already used this potion in this Game.
    PotionUsed,
    /// The Witch can only heal a Player dying tonight.
    NotDying,
    /// A Spectator (an eliminated Player) can no longer act or vote.
    Spectating,
    /// The chosen Player has been eliminated, or is not in this Game.
    NotInPlay,
    /// A Player cannot choose themselves for this.
    Yourself,
    /// The Mayor must choose among the Players tied by the Vote.
    NotTied,
    /// The Game is Paused until the Host reclaims their seat.
    Paused,
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
    Cupid,
    Thief,
    /// Has no Turn: she may peek at the table during the Werewolves' Turn.
    LittleGirl,
    /// A Camp of their own: they win only as the last Player alive.
    Assassin,
    Villager,
}

impl Role {
    /// The Camp this Role plays for, outside a mixed couple of Lovers.
    pub fn camp(self) -> Camp {
        match self {
            Role::Werewolf => Camp::Werewolves,
            Role::Assassin => Camp::Assassin,
            Role::Seer
            | Role::Witch
            | Role::Hunter
            | Role::Cupid
            | Role::Thief
            | Role::LittleGirl
            | Role::Villager => Camp::Village,
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
    /// A Werewolf and a non-Werewolf Lover: they win as the last two alive.
    Lovers,
    /// The Assassin alone.
    Assassin,
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
    /// Revealed to everyone at Elimination, unless Hidden Roles is on: then
    /// only to the Spectators, and to everyone on the victory screen.
    pub revealed_role: Option<Role>,
}

/// How many of each Role are in play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleCounts {
    pub werewolf: u8,
    pub seer: u8,
    pub witch: u8,
    pub hunter: u8,
    pub cupid: u8,
    pub thief: u8,
    pub little_girl: u8,
    pub assassin: u8,
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
            cupid: 0,
            thief: 0,
            little_girl: 0,
            assassin: 0,
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
            (Role::Cupid, self.cupid),
            (Role::Thief, self.thief),
            (Role::LittleGirl, self.little_girl),
            (Role::Assassin, self.assassin),
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
            self.cupid,
            self.thief,
            self.little_girl,
            self.assassin,
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
    pub cupid: u32,
    pub lovers: u32,
    pub thief: u32,
    pub seer: u32,
    pub werewolves: u32,
    pub witch: u32,
    pub hunter: u32,
    pub mayor_successor: u32,
    pub mayor_tie_break: u32,
    /// With Hidden Roles on, the shortest a Feigned Turn may last: it lasts
    /// a random time from this up to the Turn's own timer.
    pub feigned_turn_minimum: u32,
}

impl Timers {
    fn all(&self) -> [u32; 13] {
        [
            self.discussion,
            self.election,
            self.vote,
            self.cupid,
            self.lovers,
            self.thief,
            self.seer,
            self.werewolves,
            self.witch,
            self.hunter,
            self.mayor_successor,
            self.mayor_tie_break,
            self.feigned_turn_minimum,
        ]
    }
}

impl Default for Timers {
    fn default() -> Self {
        Self {
            discussion: 300,
            election: 60,
            vote: 60,
            cupid: 30,
            lovers: 30,
            thief: 30,
            seer: 30,
            werewolves: 60,
            witch: 30,
            hunter: 30,
            mayor_successor: 30,
            mayor_tie_break: 30,
            feigned_turn_minimum: 10,
        }
    }
}

/// Every timer lasts between these many seconds.
pub const TIMER_BOUNDS: std::ops::RangeInclusive<u32> = 10..=1800;

/// The Grace period lasts between these many seconds, like a timer.
pub const GRACE_PERIOD_BOUNDS: std::ops::RangeInclusive<u32> = TIMER_BOUNDS;

/// How long a Disconnected Player keeps their seat in the Lobby, by default.
pub const DEFAULT_GRACE_PERIOD: u32 = 60;

/// The options the Host chooses before starting a Game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    pub roles: RoleCounts,
    pub timers: Timers,
    /// Eliminated Players' Roles stay hidden from the living, and every Night
    /// Turn lasts its full time, so the Night's length reveals nothing.
    pub hidden_roles: bool,
    /// How long, in seconds, a Disconnected Player keeps their seat in the
    /// Lobby before it is freed as if they had Left.
    pub grace_period: u32,
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
    /// The two Lovers, in seat order, once Cupid has named them: only to the
    /// Lovers themselves and to the Spectators. Empty for everyone else.
    pub lovers: Vec<PlayerId>,
    /// Who the Thief is and whom they robbed, in a Game with a Thief: only to
    /// the Spectators, and to everyone on the victory screen. `None` for
    /// everyone else.
    pub theft: Option<Theft>,
    /// The visibility plan, from this Player's side: who they may see and
    /// hear right now.
    pub receives: Vec<PlayerId>,
    /// Who may see and hear this Player right now: the only ones allowed to
    /// receive their camera and microphone.
    pub audience: Vec<PlayerId>,
    /// Whether this Player may see how long the current Moment has left.
    /// With Hidden Roles on, during a Night Turn, only the Players acting in
    /// it and the Spectators do: its length could give a Feigned Turn away.
    pub sees_countdown: bool,
    /// Whether this Player may end the Day's debate now: the living Mayor,
    /// during the debate.
    pub may_end_debate: bool,
    /// Whether the Game is Paused: the Host is Disconnected during a Game
    /// not yet won. No timer runs and every Game action is refused until
    /// they reclaim their seat.
    pub paused: bool,
    /// The current Moment, while the Narrator announces it: it opens once
    /// the Table device has said its lines. Until then nothing runs in it,
    /// nobody acts in it, and the phones wait for the Narrator.
    pub announcing: Option<MomentId>,
    /// Which Night it is or was last and the Day that follows a Night shares
    /// its number
    /// 0 in the Lobby and during the Role reveal
    pub night: u32,
}

impl PlayerView {
    /// False while this Player may see and hear nobody at all.
    pub fn receives_anyone(&self) -> bool {
        !self.receives.is_empty()
    }
}

/// The Thief, and the Player they robbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Theft {
    /// The Player dealt the Thief.
    pub thief: PlayerId,
    /// Whom the Thief chose to rob, once they have. `None` if they robbed
    /// nobody, or once the Theft is cancelled at dawn because either of them
    /// died that Night.
    pub robbed: Option<PlayerId>,
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
    /// The Game opens: each Player reads their Role on their phone and
    /// confirms it. Untimed: the first Night falls once every living Player
    /// has confirmed.
    RoleReveal {
        /// How many living Players have not confirmed yet, never who.
        still_reading: u32,
        /// Whether this Player has confirmed.
        confirmed: bool,
    },
    /// The first Night: Cupid names the two Lovers.
    CupidsTurn,
    /// The first Night, right after Cupid's Turn: the Lovers wake together,
    /// each sees who their partner is (see [`PlayerView::lovers`]), and
    /// confirms.
    LoversTurn {
        /// Whether this Player has confirmed. Always false for anyone but
        /// the Lovers.
        confirmed: bool,
    },
    /// The second Night, first: the Thief chooses whom to rob, if anyone.
    ThiefsTurn,
    /// Night: the Seer inspects another living Player's Role.
    SeersTurn {
        /// What the Seer learned this Turn. Only the Seer and the Spectators
        /// see it.
        inspection: Option<Inspection>,
        /// Whether the Seer chose to inspect nobody tonight. Only the Seer
        /// and the Spectators see it.
        passed: bool,
    },
    /// Night: the Werewolves choose their Victim.
    WerewolvesTurn {
        /// Each Werewolf's current pick, live. Only the Werewolves and the
        /// Spectators see them.
        picks: Option<Vec<Pick>>,
    },
    /// Night: the Witch learns who is dying tonight and may use her potions.
    WitchsTurn {
        /// What the Witch knows and did this Turn. Only the Witch and the
        /// Spectators see it.
        witch: Option<WitchSight>,
    },
    /// The village wakes up and learns who died in the Night, in seat order,
    /// never why.
    Dawn {
        deaths: Vec<PlayerId>,
        /// Whether the Narrator says a house was robbed: at the dawn after
        /// the second Night, in a Game with a Thief, whether or not a Theft
        /// takes place.
        robbery: bool,
    },
    /// Day: the living debate.
    Discussion,
    /// The first Day, before the debate: the living elect the Mayor. Ballots
    /// stay secret until everyone has voted or the timer ends.
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
    /// A Lover dies of grief, their partner having just been eliminated.
    Grief { lover: PlayerId },
    /// The Game is over. Every Role is revealed, and the Lovers to everyone.
    /// No winner if nobody is left alive: a draw.
    Victory { winner: Option<Camp> },
}

/// The Role the Seer saw on a Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Inspection {
    pub player: PlayerId,
    pub role: Role,
}

/// The Witch's Turn, as the Witch sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WitchSight {
    /// Everyone dying tonight, in seat order, never why.
    pub dying: Vec<PlayerId>,
    /// Who she saved tonight, one of the dying.
    pub healed: Option<PlayerId>,
    /// Who she poisoned tonight.
    pub poisoned: Option<PlayerId>,
    /// Whether she has passed: she uses no more potions tonight.
    pub passed: bool,
}

/// A Werewolf's current choice of Victim. `victim` is `None` for a Werewolf
/// who passed: a pick for nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Pick {
    pub werewolf: PlayerId,
    pub victim: Option<PlayerId>,
}

/// One living Player's vote. `designated` is `None` for an abstention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Ballot {
    pub voter: PlayerId,
    pub designated: Option<PlayerId>,
}

/// Identifies one moment of a Game, so a timer armed for a moment that has
/// since ended can be told apart from the current one. A Moment being
/// announced and the same Moment once open have different ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MomentId(u32);

/// The deadline of the current moment: once `seconds` have passed since the
/// moment began, the server calls [`Engine::deadline_passed`] with `moment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    pub moment: MomentId,
    pub seconds: u32,
}

/// Identifies one Disconnection of one seat, so a Grace period armed for a
/// Player who has since come back (or left again) can be told apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DisconnectionId(u32);

impl DisconnectionId {
    /// Takes the id `counter` holds, and moves it on to the next one.
    fn next(counter: &mut u32) -> Self {
        let id = Self(*counter);
        *counter += 1;
        id
    }
}

/// A Disconnected Player's Grace period, running in the Lobby: once
/// `seconds` have passed since the server first saw `disconnection`, it calls
/// [`Engine::grace_period_passed`]. `seconds` follows the Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GracePeriod {
    pub disconnection: DisconnectionId,
    pub player: PlayerId,
    pub seconds: u32,
}

/// What the engine produces after an accepted command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outputs {
    views: Vec<(SeatToken, PlayerView)>,
    timer: Option<Timer>,
    grace_periods: Vec<GracePeriod>,
    paused: bool,
}

impl Outputs {
    /// Every Grace period running now. Empty once the Game has started.
    pub fn grace_periods(&self) -> &[GracePeriod] {
        &self.grace_periods
    }

    /// The current moment's timer, if it has one. Unchanged until the moment
    /// ends. `None` while the Game is Paused; on resume, the same timer again.
    pub fn timer(&self) -> Option<Timer> {
        self.timer
    }

    /// Whether the Game is Paused: the server keeps the time left on the
    /// current moment's timer until it is reported again.
    pub fn paused(&self) -> bool {
        self.paused
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

/// A Game has at most one Cupid: there is one pair of Lovers.
pub const MAX_CUPIDS: u8 = 1;

/// A Game has at most one Thief.
pub const MAX_THIEVES: u8 = 1;

/// A Game has at most one Little Girl.
pub const MAX_LITTLE_GIRLS: u8 = 1;

/// A Game has at most one Assassin.
pub const MAX_ASSASSINS: u8 = 1;

/// The Night the Thief wakes on, and after which the Theft takes effect.
const THIEFS_NIGHT: u32 = 2;

/// The Night whose dawn opens on the Election: the first.
const ELECTION_NIGHT: u32 = 1;

/// Long enough for a first name, short enough to fit under a video tile.
pub const MAX_NAME_CHARS: usize = 20;

#[derive(Debug)]
struct Seat {
    token: SeatToken,
    id: PlayerId,
    name: String,
    /// `None` while connected.
    disconnection: Option<DisconnectionId>,
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
    /// In seat order. `None` until Cupid names them, and in a Game without
    /// Cupid.
    lovers: Option<[PlayerId; 2]>,
    /// Which Night it is, or was last: 0 before the first Night falls.
    night: u32,
    /// The Thief and whom they chose to rob, in a Game with a Thief. Whom
    /// they robbed is cleared at dawn if the Theft is cancelled.
    theft: Option<Theft>,
    /// How long the current Turn lasts, if it is a Feigned Turn.
    feigned: Option<u32>,
    /// False while the Narrator announces the current Moment.
    open: bool,
}

/// What a Player's Elimination sets off, resolved one at a time before the
/// Game moves on (and before any victory is checked).
#[derive(Debug, Clone, Copy)]
enum Trigger {
    /// `lover` dies of grief for `mourned`, unless already dead.
    Grief {
        mourned: PlayerId,
        lover: PlayerId,
    },
    HuntersShot {
        hunter: PlayerId,
    },
    Succession {
        mayor: PlayerId,
    },
}

impl Trigger {
    /// The death triggers `dead` sets off, in the order they are resolved:
    /// their Lover dies of grief first, then a Hunter shoots, then a Mayor
    /// names a successor.
    fn set_off_by(
        dead: PlayerId,
        role: Option<Role>,
        was_mayor: bool,
        partner: Option<PlayerId>,
    ) -> Vec<Self> {
        let grief = partner.map(|lover| Trigger::Grief {
            mourned: dead,
            lover,
        });
        let shot = (role == Some(Role::Hunter)).then_some(Trigger::HuntersShot { hunter: dead });
        let succession = was_mayor.then_some(Trigger::Succession { mayor: dead });
        grief.into_iter().chain(shot).chain(succession).collect()
    }

    /// The eliminated Player this trigger belongs to.
    fn owner(self) -> PlayerId {
        match self {
            Trigger::Grief { mourned, .. } => mourned,
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
    /// Who has confirmed seeing their Role.
    RoleReveal {
        confirmed: Vec<PlayerId>,
    },
    CupidsTurn,
    /// Which Lovers have confirmed they saw their partner.
    LoversTurn {
        confirmed: Vec<PlayerId>,
    },
    ThiefsTurn,
    SeersTurn {
        inspection: Option<Inspection>,
        /// She chose to inspect nobody tonight.
        passed: bool,
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
        robbery: bool,
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
    Grief {
        lover: PlayerId,
        then: Resume,
    },
    Victory {
        winner: Option<Camp>,
    },
}

/// How long the Narrator's announcements stay on screen, in seconds.
pub const ANNOUNCEMENT_SECONDS: u32 = 10;

/// How long a Moment waits for the Table device to say it has been
/// announced, before it opens anyway. Longer than any Moment's lines.
pub const ANNOUNCEMENT_BACKSTOP_SECONDS: u32 = 30;

/// The choices made most often, tied, in the order they were first made.
/// Empty if none was.
fn top_choices<T: Copy + PartialEq>(designated: impl Iterator<Item = T>) -> Vec<T> {
    let mut counts: Vec<(T, usize)> = Vec::new();
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

/// The single choice made most often, unless several are tied (or none was
/// made).
fn single_top_choice<T: Copy + PartialEq>(designated: impl Iterator<Item = T>) -> Option<T> {
    match top_choices(designated)[..] {
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
    /// Moves on to a new Moment, which the Narrator announces before it opens.
    fn go_to(&mut self, state: MomentState) {
        self.go_to_unannounced(state);
        self.open = false;
    }

    /// Moves on to a Moment the Narrator says nothing new about, open at once.
    fn go_to_unannounced(&mut self, state: MomentState) {
        self.state = state;
        self.moment_id = MomentId(self.moment_id.0 + 1);
        self.feigned = None;
    }

    /// The Narrator has announced the current Moment: it opens, with its
    /// timer from the start.
    fn open(&mut self) {
        self.open = true;
        self.moment_id = MomentId(self.moment_id.0 + 1);
    }

    /// The current Moment, while the Narrator announces it.
    fn announcing(&self) -> Option<MomentId> {
        (!self.open).then_some(self.moment_id)
    }

    /// `None` once the Game is over: nothing is timed any more.
    fn timer(&self, timers: &Timers) -> Option<Timer> {
        if !self.open {
            return Some(Timer {
                moment: self.moment_id,
                seconds: ANNOUNCEMENT_BACKSTOP_SECONDS,
            });
        }
        let seconds = match self.state {
            // Everyone takes the time they need to read their Role.
            MomentState::RoleReveal { .. } => return None,
            MomentState::CupidsTurn => timers.cupid,
            MomentState::LoversTurn { .. } => timers.lovers,
            MomentState::ThiefsTurn => timers.thief,
            // Her Turn ends once she has read what she saw.
            MomentState::SeersTurn {
                inspection: Some(_),
                ..
            } => ANNOUNCEMENT_SECONDS,
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
            MomentState::Grief { .. } => ANNOUNCEMENT_SECONDS,
            MomentState::Victory { .. } => return None,
        };
        let seconds = self.feigned.unwrap_or(seconds);
        Some(Timer {
            moment: self.moment_id,
            seconds,
        })
    }

    fn is_over(&self) -> bool {
        matches!(self.state, MomentState::Victory { .. })
    }

    /// The other Lover, if `id` is one of the Lovers.
    fn partner_of(&self, id: PlayerId) -> Option<PlayerId> {
        let [first, second] = self.lovers?;
        match id {
            _ if id == first => Some(second),
            _ if id == second => Some(first),
            _ => None,
        }
    }

    /// This moment as `viewer` may see it.
    /// `living`: how many Players are alive.
    fn moment(&self, viewer: &Seat, living: usize) -> Moment {
        match &self.state {
            MomentState::RoleReveal { confirmed } => Moment::RoleReveal {
                still_reading: (living - confirmed.len()) as u32,
                confirmed: confirmed.contains(&viewer.id),
            },
            MomentState::CupidsTurn => Moment::CupidsTurn,
            MomentState::LoversTurn { confirmed } => Moment::LoversTurn {
                confirmed: confirmed.contains(&viewer.id),
            },
            MomentState::ThiefsTurn => Moment::ThiefsTurn,
            MomentState::SeersTurn { inspection, passed } => {
                // The Seer and the Spectators see it.
                let sees = viewer.role == Some(Role::Seer) || !viewer.alive;
                Moment::SeersTurn {
                    inspection: sees.then_some(*inspection).flatten(),
                    passed: sees && *passed,
                }
            }
            MomentState::WerewolvesTurn { picks } => Moment::WerewolvesTurn {
                // The Werewolves and the Spectators see them.
                picks: (viewer.role == Some(Role::Werewolf) || !viewer.alive)
                    .then(|| picks.clone()),
            },
            MomentState::WitchsTurn { sight } => Moment::WitchsTurn {
                // The Witch and the Spectators see it.
                witch: (viewer.role == Some(Role::Witch) || !viewer.alive).then(|| sight.clone()),
            },
            MomentState::Dawn { deaths, robbery } => Moment::Dawn {
                deaths: deaths.clone(),
                robbery: *robbery,
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
            MomentState::Grief { lover, .. } => Moment::Grief { lover: *lover },
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
    hidden_roles: bool,
    grace_period: u32,
    next_disconnection: u32,
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
            hidden_roles: false,
            grace_period: DEFAULT_GRACE_PERIOD,
            next_disconnection: 1,
            game: None,
            randomness: Box::new(randomness),
        }
    }

    pub fn handle(&mut self, seat: &SeatToken, command: Command) -> Result<Outputs, Rejection> {
        let comes_and_goes = matches!(
            command,
            Command::Join { .. } | Command::Leave | Command::Disconnect
        );
        if self.paused() && !comes_and_goes {
            // Someone without a seat is told so first.
            self.seat_of(seat)?;
            return Err(Rejection::Paused);
        }
        let acts_in_the_moment = !comes_and_goes
            && !matches!(
                command,
                Command::UpdateSettings { .. } | Command::Start | Command::Announced { .. }
            );
        if acts_in_the_moment && self.announcing().is_some() {
            self.seat_of(seat)?;
            return Err(Rejection::NotNow);
        }
        match command {
            Command::Join { name } => {
                let name = name.trim().to_string();
                if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
                    return Err(Rejection::InvalidName);
                }
                if let Some(reclaimed) = self.seat_mut(seat) {
                    reclaimed.name = name;
                    reclaimed.disconnection = None;
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
                    disconnection: None,
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
                let next = &mut self.next_disconnection;
                let seat = self
                    .seats
                    .iter_mut()
                    .find(|s| &s.token == seat)
                    .ok_or(Rejection::NotSeated)?;
                if seat.disconnection.is_none() {
                    seat.disconnection = Some(DisconnectionId::next(next));
                }
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
                    || settings.roles.cupid > MAX_CUPIDS
                    || settings.roles.thief > MAX_THIEVES
                    || settings.roles.little_girl > MAX_LITTLE_GIRLS
                    || settings.roles.assassin > MAX_ASSASSINS
                    || !timers_valid
                    || !GRACE_PERIOD_BOUNDS.contains(&settings.grace_period)
                {
                    return Err(Rejection::InvalidSettings);
                }
                self.custom_roles = Some(settings.roles).filter(|&r| r != self.suggested_roles());
                self.timers = settings.timers;
                self.hidden_roles = settings.hidden_roles;
                self.grace_period = settings.grace_period;
            }
            Command::Start => {
                self.require_host(seat)?;
                if let Some(blocker) = self.start_blocker() {
                    return Err(blocker);
                }
                self.deal();
                let theft = self
                    .seats
                    .iter()
                    .find(|s| s.role == Some(Role::Thief))
                    .map(|s| Theft {
                        thief: s.id,
                        robbed: None,
                    });
                self.game = Some(Game {
                    state: MomentState::RoleReveal {
                        confirmed: Vec::new(),
                    },
                    moment_id: MomentId(0),
                    potions: Potions {
                        healing: true,
                        poison: true,
                    },
                    triggers: VecDeque::new(),
                    mayor: None,
                    lovers: None,
                    night: 0,
                    theft,
                    feigned: None,
                    open: false,
                });
            }
            Command::ConfirmRole => {
                let player = self.require_living(seat)?;
                let living = self.living().count();
                let MomentState::RoleReveal { confirmed } = &mut self.game_mut().state else {
                    return Err(Rejection::NotNow);
                };
                if !confirmed.contains(&player) {
                    confirmed.push(player);
                }
                if confirmed.len() == living {
                    let night = self.nightfall();
                    self.game_mut().go_to(night);
                    self.feign_if_idle();
                }
            }
            Command::ChooseLovers { lovers } => {
                self.require_cupids_turn(seat)?;
                let &[first, second] = &lovers[..] else {
                    return Err(Rejection::NotTwoPlayers);
                };
                if first == second {
                    return Err(Rejection::NotTwoPlayers);
                }
                self.require_in_play(Some(first))?;
                self.require_in_play(Some(second))?;
                self.bind_lovers(first, second);
            }
            Command::ConfirmPartner => {
                let lover = self.require_living(seat)?;
                let is_lover = self.is_lover(self.seat_of(seat)?);
                let MomentState::LoversTurn { confirmed } = &mut self.game_mut().state else {
                    return Err(Rejection::NotNow);
                };
                if !is_lover {
                    return Err(Rejection::NotYourTurn);
                }
                if !confirmed.contains(&lover) {
                    confirmed.push(lover);
                }
                if confirmed.len() == 2 {
                    self.end_lovers_turn();
                }
            }
            Command::Rob { player } => {
                let thief = self.require_thiefs_turn(seat)?;
                self.require_in_play(Some(player))?;
                if player == thief {
                    return Err(Rejection::Yourself);
                }
                if let Some(theft) = &mut self.game_mut().theft {
                    theft.robbed = Some(player);
                }
                self.end_thiefs_turn();
            }
            Command::Inspect { player } => {
                let seer = self.require_seers_turn(seat)?;
                self.require_in_play(Some(player))?;
                if player == seer {
                    return Err(Rejection::Yourself);
                }
                let role = self
                    .living()
                    .find(|s| s.id == player)
                    .and_then(|s| s.role)
                    .expect("a living Player of this Game has a Role");
                // Her action is complete: a new Moment, timed only for her
                // to read what she saw. The Narrator has nothing new to say.
                self.game_mut().go_to_unannounced(MomentState::SeersTurn {
                    inspection: Some(Inspection { player, role }),
                    passed: false,
                });
            }
            Command::PickVictim { victim } => {
                let werewolf = self.require_werewolves_turn(seat)?;
                self.require_in_play(Some(victim))?;
                self.pick(werewolf, Some(victim));
            }
            Command::Heal { player } => {
                self.require_witchs_turn(seat)?;
                if !self.game_mut().potions.healing {
                    return Err(Rejection::PotionUsed);
                }
                let sight = self.witch_sight_mut();
                if !sight.dying.contains(&player) {
                    return Err(Rejection::NotDying);
                }
                sight.healed = Some(player);
                self.game_mut().potions.healing = false;
                self.end_witchs_turn_if_done();
            }
            Command::Poison { player } => {
                self.require_witchs_turn(seat)?;
                if !self.game_mut().potions.poison {
                    return Err(Rejection::PotionUsed);
                }
                self.require_in_play(Some(player))?;
                self.witch_sight_mut().poisoned = Some(player);
                self.game_mut().potions.poison = false;
                self.end_witchs_turn_if_done();
            }
            Command::Pass => match self.state() {
                Some(MomentState::WerewolvesTurn { .. }) => {
                    let werewolf = self.require_werewolves_turn(seat)?;
                    self.pick(werewolf, None);
                }
                Some(MomentState::ThiefsTurn) => {
                    self.require_thiefs_turn(seat)?;
                    self.end_thiefs_turn();
                }
                Some(MomentState::SeersTurn { .. }) => {
                    self.require_seers_turn(seat)?;
                    self.game_mut().go_to(Self::werewolves_turn());
                }
                Some(MomentState::WitchsTurn { .. }) => {
                    self.require_witchs_turn(seat)?;
                    self.witch_sight_mut().passed = true;
                    self.end_witchs_turn_if_done();
                }
                // The same as letting the time run out: the shot is lost.
                Some(MomentState::HuntersShot { .. }) => {
                    let (hunter, then) = self.require_hunters_shot(seat)?;
                    self.game_mut().go_to(MomentState::ShotResult {
                        hunter,
                        shot: None,
                        then,
                    });
                }
                // The same as letting the time run out: nobody is eliminated.
                Some(MomentState::TieBreak { .. }) => {
                    let (ballots, _) = self.require_tie_break(seat)?;
                    self.announce_vote(ballots, None);
                }
                // The eliminated Mayor must name a successor.
                Some(&MomentState::Succession { mayor, .. }) => {
                    let namer = self.seat_of(seat)?.id;
                    return Err(if namer == mayor {
                        Rejection::NotNow
                    } else {
                        Rejection::NotYourTurn
                    });
                }
                _ => {
                    self.require_living(seat)?;
                    return Err(Rejection::NotNow);
                }
            },
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
            Command::EndDebate => {
                self.require_debate_leader(seat)?;
                self.end_debate();
            }
            Command::BreakTie { player } => {
                let (ballots, tied) = self.require_tie_break(seat)?;
                if !tied.contains(&player) {
                    return Err(Rejection::NotTied);
                }
                self.announce_vote(ballots, Some(player));
            }
            Command::Shoot { player } => {
                let (hunter, then) = self.require_hunters_shot(seat)?;
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
            Command::Announced { moment } => {
                self.require_host(seat)?;
                if self.announcing() != Some(moment) {
                    return Err(Rejection::NotNow);
                }
                self.game_mut().open();
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
                    // Back in the Lobby, a Disconnected Player's Grace period
                    // starts now.
                    if seat.disconnection.is_some() {
                        seat.disconnection =
                            Some(DisconnectionId::next(&mut self.next_disconnection));
                    }
                }
            }
        }
        Ok(self.outputs())
    }

    /// Called by the server when the timer armed for `moment` fires. Refused
    /// if that moment has already ended, or while the Game is Paused.
    pub fn deadline_passed(&mut self, moment: MomentId) -> Result<Outputs, Rejection> {
        if self.paused() {
            return Err(Rejection::Paused);
        }
        let game = self.game.as_ref().ok_or(Rejection::NotNow)?;
        if game.moment_id != moment {
            return Err(Rejection::NotNow);
        }
        // The Table device never said it was announced: it opens anyway.
        if !game.open {
            self.game_mut().open();
            return Ok(self.outputs());
        }
        match &game.state {
            // Untimed: no deadline ever passes.
            MomentState::RoleReveal { .. } => return Err(Rejection::NotNow),
            // Cupid named nobody in time: the Lovers are drawn.
            MomentState::CupidsTurn => {
                let mut living: Vec<PlayerId> = self.living().map(|s| s.id).collect();
                let first = self.draw(&living);
                living.retain(|&id| id != first);
                let second = self.draw(&living);
                self.bind_lovers(first, second);
            }
            MomentState::LoversTurn { .. } => self.end_lovers_turn(),
            // The Thief robbed nobody in time.
            MomentState::ThiefsTurn => self.end_thiefs_turn(),
            MomentState::SeersTurn { .. } => self.game_mut().go_to(Self::werewolves_turn()),
            MomentState::WerewolvesTurn { .. } => {
                let victim = self.most_picked();
                self.end_werewolves_turn(victim);
            }
            MomentState::WitchsTurn { .. } => self.end_witchs_turn(),
            MomentState::Dawn { .. } => self.resolve_triggers(Resume::Day),
            MomentState::Discussion => self.end_debate(),
            MomentState::ElectionResult { .. } => self.game_mut().go_to(MomentState::Discussion),
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
            &MomentState::Grief { then, .. } => self.resolve_triggers(then),
            MomentState::Victory { .. } => return Err(Rejection::NotNow),
        }
        Ok(self.outputs())
    }

    /// Called by the server once `disconnection` has lasted `elapsed` seconds.
    /// In the Lobby, if that is the whole Grace period, the seat is freed as
    /// if the Player had Left. Refused if the Player has since come back,
    /// their seat is already freed, the Game has started, or the Grace period
    /// is not over yet.
    pub fn grace_period_passed(
        &mut self,
        disconnection: DisconnectionId,
        elapsed: u32,
    ) -> Result<Outputs, Rejection> {
        if self.game.is_some() || elapsed < self.grace_period {
            return Err(Rejection::NotNow);
        }
        let index = self
            .seats
            .iter()
            .position(|s| s.disconnection == Some(disconnection))
            .ok_or(Rejection::NotNow)?;
        self.seats.remove(index);
        Ok(self.outputs())
    }

    /// Nobody is seated any more: the Lobby can be forgotten.
    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }

    /// Refused unless this seat's phone is the Table device of a Game in
    /// progress: the only one the server voices the Narrator's live lines for.
    pub fn require_table_device(&self, token: &SeatToken) -> Result<(), Rejection> {
        self.require_host(token)?;
        if self.game.as_ref().is_none_or(Game::is_over) {
            return Err(Rejection::NotNow);
        }
        Ok(())
    }

    /// The current Moment, while the Narrator announces it.
    fn announcing(&self) -> Option<MomentId> {
        self.game.as_ref().and_then(Game::announcing)
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

    /// The Player on this seat is the living Cupid, and it is their Turn.
    fn require_cupids_turn(&self, token: &SeatToken) -> Result<(), Rejection> {
        self.require_living(token)?;
        let is_cupid = self.seat_of(token)?.role == Some(Role::Cupid);
        if !matches!(self.state(), Some(MomentState::CupidsTurn)) {
            Err(Rejection::NotNow)
        } else if !is_cupid {
            Err(Rejection::NotYourTurn)
        } else {
            Ok(())
        }
    }

    /// The Player on this seat is the living Thief, and it is their Turn.
    fn require_thiefs_turn(&self, token: &SeatToken) -> Result<PlayerId, Rejection> {
        let thief = self.require_living(token)?;
        let is_thief = self.seat_of(token)?.role == Some(Role::Thief);
        if !matches!(self.state(), Some(MomentState::ThiefsTurn)) {
            Err(Rejection::NotNow)
        } else if !is_thief {
            Err(Rejection::NotYourTurn)
        } else {
            Ok(thief)
        }
    }

    /// The Player on this seat is the living Seer, it is her Turn, and she
    /// has neither inspected nor passed yet: she acts only once a Night.
    fn require_seers_turn(&self, token: &SeatToken) -> Result<PlayerId, Rejection> {
        let seer = self.require_living(token)?;
        let is_seer = self.seat_of(token)?.role == Some(Role::Seer);
        let Some(MomentState::SeersTurn { inspection, passed }) = self.state() else {
            return Err(Rejection::NotNow);
        };
        if !is_seer {
            Err(Rejection::NotYourTurn)
        } else if inspection.is_some() || *passed {
            Err(Rejection::NotNow)
        } else {
            Ok(seer)
        }
    }

    /// The Player on this seat is a living Werewolf, and it is their Turn.
    fn require_werewolves_turn(&self, token: &SeatToken) -> Result<PlayerId, Rejection> {
        let werewolf = self.require_living(token)?;
        let is_werewolf = self.seat_of(token)?.role == Some(Role::Werewolf);
        if !matches!(self.state(), Some(MomentState::WerewolvesTurn { .. })) {
            Err(Rejection::NotNow)
        } else if !is_werewolf {
            Err(Rejection::NotYourTurn)
        } else {
            Ok(werewolf)
        }
    }

    /// The Player on this seat is the living Witch, it is her Turn, and she
    /// has not passed.
    fn require_witchs_turn(&self, token: &SeatToken) -> Result<(), Rejection> {
        self.require_living(token)?;
        let is_witch = self.seat_of(token)?.role == Some(Role::Witch);
        let Some(MomentState::WitchsTurn { sight }) = self.state() else {
            return Err(Rejection::NotNow);
        };
        if !is_witch {
            Err(Rejection::NotYourTurn)
        } else if sight.passed {
            Err(Rejection::NotNow)
        } else {
            Ok(())
        }
    }

    /// The Player on this seat is the eliminated Hunter, and it is his shot.
    /// Returns him, and where the Game goes once his shot is resolved.
    fn require_hunters_shot(&self, token: &SeatToken) -> Result<(PlayerId, Resume), Rejection> {
        let shooter = self.seat_of(token)?.id;
        let Some(&MomentState::HuntersShot { hunter, then }) = self.state() else {
            return Err(Rejection::NotNow);
        };
        if shooter != hunter {
            return Err(Rejection::NotYourTurn);
        }
        Ok((hunter, then))
    }

    /// The Player on this seat is the living Mayor, and the Vote is tied.
    /// Returns the Vote's ballots and the Players tied.
    fn require_tie_break(
        &self,
        token: &SeatToken,
    ) -> Result<(Vec<Ballot>, Vec<PlayerId>), Rejection> {
        let chooser = self.require_living(token)?;
        let Some(MomentState::TieBreak { ballots, tied }) = self.state() else {
            return Err(Rejection::NotNow);
        };
        if self.mayor() != Some(chooser) {
            return Err(Rejection::NotYourTurn);
        }
        Ok((ballots.clone(), tied.clone()))
    }

    /// It is the Day's debate, and the Player on this seat may end it: the
    /// living Mayor.
    fn require_debate_leader(&self, token: &SeatToken) -> Result<(), Rejection> {
        let player = self.seat_of(token)?.id;
        let debate = matches!(self.state(), Some(MomentState::Discussion));
        if debate && self.debate_leader() == Some(player) {
            return Ok(());
        }
        self.require_living(token)?;
        if !debate {
            Err(Rejection::NotNow)
        } else {
            Err(Rejection::NotYourTurn)
        }
    }

    /// The chosen Player, if any, is a living Player of this Game.
    fn require_in_play(&self, chosen: Option<PlayerId>) -> Result<(), Rejection> {
        match chosen {
            Some(id) if !self.is_alive(id) => Err(Rejection::NotInPlay),
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
            camp: self.camp_of(seat),
            fellow_werewolves,
            potions,
        })
    }

    /// Records (or changes) `werewolf`'s pick: a Victim, or nobody. Their
    /// Turn ends once every living Werewolf agrees.
    fn pick(&mut self, werewolf: PlayerId, victim: Option<PlayerId>) {
        let picks = self.picks_mut();
        picks.retain(|p| p.werewolf != werewolf);
        picks.push(Pick { werewolf, victim });
        if let Some(victim) = self.agreed_pick() {
            self.end_werewolves_turn(victim);
        }
    }

    /// What every living Werewolf picked, if they all agree: the same Victim,
    /// or nobody if they all passed.
    fn agreed_pick(&self) -> Option<Option<PlayerId>> {
        let picks = self.picks()?;
        let mut choices = self
            .living()
            .filter(|s| s.role == Some(Role::Werewolf))
            .map(|s| picks.iter().find(|p| p.werewolf == s.id).map(|p| p.victim));
        let first = choices.next()??;
        choices.all(|c| c == Some(first)).then_some(first)
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

    /// Whether `role` has a Turn tonight. With Hidden Roles off, the Turn of a
    /// Role nobody living holds is skipped; with it on, every Role dealt in
    /// this Game keeps its Turn, so the Night's length reveals nothing.
    fn wakes(&self, role: Role) -> bool {
        if self.hidden_roles {
            self.seats.iter().any(|s| s.role == Some(role))
        } else {
            self.has_living_holder(role)
        }
    }

    /// Whether a living Player holds `role`.
    fn has_living_holder(&self, role: Role) -> bool {
        self.living().any(|s| s.role == Some(role))
    }

    /// The next Night falls: its first Turn. The Night order is Cupid and
    /// then the Lovers, on the first Night only, the Thief on the second
    /// Night only, then the Seer, then the Werewolves. Nobody can be dead on
    /// the first Night, so neither Cupid's nor the Lovers' Turn is ever
    /// Feigned.
    fn nightfall(&mut self) -> MomentState {
        let game = self.game_mut();
        game.night += 1;
        let night = game.night;
        if night == 1 && self.wakes(Role::Cupid) {
            MomentState::CupidsTurn
        } else if night == THIEFS_NIGHT && self.wakes(Role::Thief) {
            MomentState::ThiefsTurn
        } else {
            self.seers_turn_or_after()
        }
    }

    /// The Seer's Turn, unless it is skipped: then the Werewolves'.
    fn seers_turn_or_after(&self) -> MomentState {
        if self.wakes(Role::Seer) {
            MomentState::SeersTurn {
                inspection: None,
                passed: false,
            }
        } else {
            Self::werewolves_turn()
        }
    }

    /// Cupid has named `first` and `second`: they are the Lovers for the
    /// rest of the Game, and wake together right away.
    fn bind_lovers(&mut self, first: PlayerId, second: PlayerId) {
        let game = self.game_mut();
        game.lovers = Some([first.min(second), first.max(second)]);
        game.go_to(MomentState::LoversTurn {
            confirmed: Vec::new(),
        });
    }

    /// The Lovers go back to sleep, and the Night goes on.
    fn end_lovers_turn(&mut self) {
        let next = self.seers_turn_or_after();
        self.game_mut().go_to(next);
        self.feign_if_idle();
    }

    /// The Thief goes back to sleep, and the Night goes on.
    fn end_thiefs_turn(&mut self) {
        let next = self.seers_turn_or_after();
        self.game_mut().go_to(next);
        self.feign_if_idle();
    }

    /// The Theft takes effect, at the dawn after the second Night, once its
    /// deaths are resolved: if the Thief and the Player they robbed are both
    /// alive, the Thief takes their Role as it stands and they hold a
    /// Villager card. Otherwise the Theft is cancelled, and a living Thief
    /// without a Role plays as a Villager. It happens once: afterwards the
    /// Thief no longer holds the Thief card.
    fn carry_out_theft(&mut self) {
        let Some(Theft { thief, robbed }) = self.game.as_ref().and_then(|g| g.theft) else {
            return;
        };
        let robs = self
            .living()
            .any(|s| s.id == thief && s.role == Some(Role::Thief));
        if !robs && self.is_alive(thief) {
            return;
        }
        let robbed = robbed.filter(|&r| robs && self.is_alive(r));
        if let Some(theft) = &mut self.game_mut().theft {
            theft.robbed = robbed;
        }
        if !robs {
            return;
        }
        let mut taken = Role::Villager;
        if let Some(seat) = self.seats.iter_mut().find(|s| Some(s.id) == robbed) {
            taken = seat.role.replace(Role::Villager).unwrap_or(Role::Villager);
        }
        if let Some(seat) = self.seats.iter_mut().find(|s| s.id == thief) {
            seat.role = Some(taken);
        }
    }

    /// The Thief and whom they robbed, if `viewer` may know: a Spectator, or
    /// anyone once the Game is over.
    fn theft_seen_by(&self, viewer: &Seat) -> Option<Theft> {
        let game = self.game.as_ref()?;
        game.theft
            .filter(|_| self.spectating(viewer) || game.is_over())
    }

    /// The two Lovers, if `viewer` may know who they are: a Lover, or a
    /// Spectator.
    fn lovers_seen_by(&self, viewer: &Seat) -> Vec<PlayerId> {
        match self.game.as_ref().and_then(|g| g.lovers) {
            Some(lovers) if self.is_lover(viewer) || self.spectating(viewer) || self.is_over() => {
                lovers.to_vec()
            }
            _ => Vec::new(),
        }
    }

    /// Whether the Game is over, on the victory screen.
    fn is_over(&self) -> bool {
        self.game.as_ref().is_some_and(Game::is_over)
    }

    /// The Camp `seat` plays for: the Lovers Camp in a mixed couple (one a
    /// Werewolf, the other not), otherwise their Role's.
    fn camp_of(&self, seat: &Seat) -> Camp {
        let werewolf = |s: &Seat| s.role == Some(Role::Werewolf);
        let partner = self
            .game
            .as_ref()
            .and_then(|g| g.partner_of(seat.id))
            .and_then(|id| self.seats.iter().find(|s| s.id == id));
        match (seat.role, partner) {
            (_, Some(partner)) if werewolf(seat) != werewolf(partner) => Camp::Lovers,
            (Some(role), _) => role.camp(),
            (None, _) => Camp::Village,
        }
    }

    /// Whether `seat` is one of the Lovers.
    fn is_lover(&self, seat: &Seat) -> bool {
        self.game
            .as_ref()
            .and_then(|g| g.lovers)
            .is_some_and(|l| l.contains(&seat.id))
    }

    /// The Werewolves' Turn, before anyone has picked.
    fn werewolves_turn() -> MomentState {
        MomentState::WerewolvesTurn { picks: Vec::new() }
    }

    /// The Werewolves have chosen their Victim, if any. The Witch wakes up
    /// next, unless her Turn is skipped: then the Night is over.
    fn end_werewolves_turn(&mut self, victim: Option<PlayerId>) {
        let dying: Vec<PlayerId> = victim.into_iter().collect();
        if self.wakes(Role::Witch) {
            self.game_mut().go_to(MomentState::WitchsTurn {
                sight: WitchSight {
                    dying,
                    healed: None,
                    poisoned: None,
                    passed: false,
                },
            });
            self.feign_if_idle();
            self.end_witchs_turn_if_done();
        } else {
            self.end_night(dying);
        }
    }

    /// The living Witch's Turn ends as soon as she has passed, or has no
    /// potion left that she could use tonight. A Feigned Turn lasts its
    /// whole drawn time instead.
    fn end_witchs_turn_if_done(&mut self) {
        let Some(game) = &self.game else {
            unreachable!("the Game is running")
        };
        let MomentState::WitchsTurn { sight } = &game.state else {
            unreachable!("only during the Witch's Turn")
        };
        let done = sight.passed || !self.witch_has_a_usable_potion();
        if done && self.has_living_holder(Role::Witch) && game.feigned.is_none() {
            self.end_witchs_turn();
        }
    }

    /// During the Witch's Turn: whether a potion she still has could be
    /// used tonight.
    fn witch_has_a_usable_potion(&self) -> bool {
        let Some(game) = &self.game else {
            unreachable!("the Game is running")
        };
        let MomentState::WitchsTurn { sight } = &game.state else {
            unreachable!("only during the Witch's Turn")
        };
        let can_heal = game.potions.healing && !sight.dying.is_empty() && sight.healed.is_none();
        can_heal || game.potions.poison
    }

    /// With Hidden Roles on, a Turn where nothing can happen is feigned, so
    /// its length reveals nothing: the Thief's, the Seer's or the Witch's
    /// Turn when they are dead, or the Witch's when she has no potion she
    /// could use tonight. It lasts a random time, from the Feigned Turn
    /// minimum up to the Turn's own timer.
    fn feign_if_idle(&mut self) {
        if !self.hidden_roles {
            return;
        }
        let longest = match self.state() {
            Some(MomentState::ThiefsTurn) if !self.has_living_holder(Role::Thief) => {
                self.timers.thief
            }
            Some(MomentState::SeersTurn { .. }) if !self.has_living_holder(Role::Seer) => {
                self.timers.seer
            }
            Some(MomentState::WitchsTurn { .. })
                if !self.has_living_holder(Role::Witch) || !self.witch_has_a_usable_potion() =>
            {
                self.timers.witch
            }
            _ => return,
        };
        let shortest = self.timers.feigned_turn_minimum.min(longest);
        let drawn = self.randomness.below((longest - shortest + 1) as usize) as u32;
        self.game_mut().feigned = Some(shortest + drawn);
    }

    /// The Witch's Turn is over, and with it the Night: everyone dying
    /// tonight dies but the one she saved, and so does whoever she poisoned.
    fn end_witchs_turn(&mut self) {
        let MomentState::WitchsTurn { sight } = &self.game_mut().state else {
            unreachable!("only during the Witch's Turn")
        };
        let killed = sight.dying.iter().filter(|&&p| Some(p) != sight.healed);
        let deaths = killed.copied().chain(sight.poisoned).collect();
        self.end_night(deaths);
    }

    /// The Night is over: `deaths` die, and the village wakes up. They are
    /// announced in seat order, so the order never tells what killed whom.
    fn end_night(&mut self, mut deaths: Vec<PlayerId>) {
        deaths.sort();
        deaths.dedup();
        for &id in &deaths {
            self.eliminate(id);
        }
        let game = self.game_mut();
        let robbery = game.night == THIEFS_NIGHT && game.theft.is_some();
        game.go_to(MomentState::Dawn { deaths, robbery });
    }

    /// The Player most picked by the Werewolves, unless several choices are
    /// tied. Nobody, the pick of a Werewolf who passed, counts as a choice.
    fn most_picked(&self) -> Option<PlayerId> {
        single_top_choice(self.picks()?.iter().map(|p| p.victim)).flatten()
    }

    /// The Election is over: it is revealed, and the most-voted Player becomes
    /// Mayor. A tie, or an Election nobody voted in, is settled at random
    /// among the tied, or among all the living.
    fn end_election(&mut self) {
        let MomentState::Election { ballots } = &self.game_mut().state else {
            unreachable!("only an Election ends")
        };
        let ballots = ballots.clone();
        let mut candidates = top_choices(ballots.iter().filter_map(|b| b.designated));
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

    /// The Day's debate is over: the Vote follows.
    fn end_debate(&mut self) {
        self.game_mut().go_to(MomentState::Vote {
            ballots: Vec::new(),
        });
    }

    /// Who may end the Day's debate early: the living Mayor.
    fn debate_leader(&self) -> Option<PlayerId> {
        self.mayor().filter(|&mayor| self.is_alive(mayor))
    }

    /// The Vote is over: it is revealed, and the most-designated Player dies.
    /// A tie goes to the living Mayor, who chooses among the tied.
    fn end_vote(&mut self) {
        let MomentState::Vote { ballots } = &self.game_mut().state else {
            unreachable!("only a Vote ends")
        };
        let ballots = ballots.clone();
        let tied = top_choices(ballots.iter().filter_map(|b| b.designated));
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
            // A Lover already dead (both died at once, or the Hunter shot
            // them) does not die again.
            Some(Trigger::Grief { lover, .. }) if !self.is_alive(lover) => {
                self.resolve_triggers(then)
            }
            Some(Trigger::Grief { lover, .. }) => {
                self.eliminate(lover);
                self.game_mut().go_to(MomentState::Grief { lover, then });
            }
            Some(Trigger::HuntersShot { hunter }) => self
                .game_mut()
                .go_to(MomentState::HuntersShot { hunter, then }),
            // Once a Camp has won (or nobody is left alive), a successor
            // would change nothing.
            Some(Trigger::Succession { .. }) if self.decided() => self.resolve_triggers(then),
            Some(Trigger::Succession { mayor }) => self
                .game_mut()
                .go_to(MomentState::Succession { mayor, then }),
            None => match then {
                Resume::Day => {
                    let night = self.game.as_ref().map_or(0, |g| g.night);
                    if night == THIEFS_NIGHT {
                        self.carry_out_theft();
                    }
                    // The first Day elects the Mayor before its debate (the
                    // Mayor is elected then only).
                    let day = if night == ELECTION_NIGHT {
                        MomentState::Election {
                            ballots: Vec::new(),
                        }
                    } else {
                        MomentState::Discussion
                    };
                    self.unless_decided(day);
                }
                Resume::Night => {
                    let night = self.nightfall();
                    self.unless_decided(night);
                    self.feign_if_idle();
                }
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

    /// Moves on to `next`, unless the Game is decided: then it is over.
    fn unless_decided(&mut self, next: MomentState) {
        let next = if self.decided() {
            MomentState::Victory {
                winner: self.winner(),
            }
        } else {
            next
        };
        self.game_mut().go_to(next);
    }

    /// Whether the Game is over: a Camp has won, or nobody is left alive.
    fn decided(&self) -> bool {
        self.winner().is_some() || self.living().next().is_none()
    }

    /// The Village wins once every Werewolf is dead, a Werewolf Lover
    /// included, which takes precedence; a mixed couple of Lovers wins as the
    /// last two alive; the Werewolves win once they are at least as many as
    /// everyone else, the Lovers counted among everyone else. Nobody wins
    /// while nobody is alive.
    fn winner(&self) -> Option<Camp> {
        let living = self.living().count();
        let in_camp = |camp| self.living().filter(|s| self.camp_of(s) == camp).count();
        let werewolves = in_camp(Camp::Werewolves);
        if living == 0 {
            None
        } else if !self.living().any(|s| s.role == Some(Role::Werewolf)) {
            Some(Camp::Village)
        } else if living == 2 && in_camp(Camp::Lovers) == 2 {
            Some(Camp::Lovers)
        } else if werewolves >= living - werewolves {
            Some(Camp::Werewolves)
        } else {
            None
        }
    }

    fn living(&self) -> impl Iterator<Item = &Seat> {
        self.seats.iter().filter(|s| s.alive)
    }

    fn is_alive(&self, id: PlayerId) -> bool {
        self.living().any(|s| s.id == id)
    }

    /// `id` dies, setting off their death triggers, if they have any.
    fn eliminate(&mut self, id: PlayerId) {
        let Some(seat) = self.seats.iter_mut().find(|s| s.id == id) else {
            return;
        };
        seat.alive = false;
        let role = seat.role;
        let game = self.game_mut();
        let triggers = Trigger::set_off_by(id, role, game.mayor == Some(id), game.partner_of(id));
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

    /// The Host is Disconnected during a Game not yet won: the table has no
    /// Narrator voice.
    fn paused(&self) -> bool {
        let host_away = self
            .seats
            .first()
            .is_some_and(|h| h.disconnection.is_some());
        host_away && self.game.as_ref().is_some_and(|g| !g.is_over())
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
        } else if self.seats.iter().any(|s| s.disconnection.is_some()) {
            Some(Rejection::PlayersDisconnected)
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
            hidden_roles: self.hidden_roles,
            grace_period: self.grace_period,
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
            // Cupid, the Thief, the Seer and the Witch act alone, seeing and
            // heard by no one.
            Some(
                MomentState::CupidsTurn
                | MomentState::ThiefsTurn
                | MomentState::SeersTurn { .. }
                | MomentState::WitchsTurn { .. },
            ) => false,
            // The Lovers wake together.
            Some(MomentState::LoversTurn { .. }) => {
                self.is_lover(receiver) && self.is_lover(sender)
            }
            Some(MomentState::WerewolvesTurn { .. }) => werewolf(receiver) && werewolf(sender),
            Some(
                MomentState::RoleReveal { .. }
                | MomentState::Dawn { .. }
                | MomentState::Discussion
                | MomentState::Election { .. }
                | MomentState::ElectionResult { .. }
                | MomentState::Vote { .. }
                | MomentState::TieBreak { .. }
                | MomentState::VoteResult { .. }
                | MomentState::HuntersShot { .. }
                | MomentState::ShotResult { .. }
                | MomentState::Succession { .. }
                | MomentState::SuccessionResult { .. }
                | MomentState::Grief { .. },
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

    /// Whether `viewer` may see how long the current Moment has left.
    fn sees_countdown(&self, viewer: &Seat) -> bool {
        // The backstop of an announcement is no countdown.
        if self.announcing().is_some() {
            return false;
        }
        let has = |role: Role| viewer.role == Some(role);
        let acting = match self.state() {
            Some(MomentState::CupidsTurn) => has(Role::Cupid),
            Some(MomentState::LoversTurn { .. }) => self.is_lover(viewer),
            Some(MomentState::ThiefsTurn) => has(Role::Thief),
            Some(MomentState::SeersTurn { .. }) => has(Role::Seer),
            Some(MomentState::WerewolvesTurn { .. }) => has(Role::Werewolf),
            Some(MomentState::WitchsTurn { .. }) => has(Role::Witch),
            _ => return true,
        };
        !self.hidden_roles || !viewer.alive || acting
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
    /// the eliminated (unless Hidden Roles is on), and every Role for a
    /// Spectator or once the Game is over.
    fn players_seen_by(&self, viewer: &Seat) -> Vec<PlayerSummary> {
        let sees_every_role = self
            .game
            .as_ref()
            .is_some_and(|g| g.is_over() || self.spectating(viewer));
        let sees_dead_roles = sees_every_role || !self.hidden_roles;
        self.seats
            .iter()
            .map(|s| PlayerSummary {
                id: s.id,
                name: s.name.clone(),
                connected: s.disconnection.is_none(),
                alive: s.alive,
                revealed_role: s
                    .role
                    .filter(|_| sees_every_role || (!s.alive && sees_dead_roles)),
            })
            .collect()
    }

    fn outputs(&self) -> Outputs {
        // The Host is whoever has been seated the longest.
        let Some(host) = self.seats.first().map(|s| s.id) else {
            return Outputs {
                views: Vec::new(),
                timer: None,
                grace_periods: Vec::new(),
                paused: false,
            };
        };
        let living = self.living().count();
        let paused = self.paused();
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
                    moment: self.game.as_ref().map(|g| g.moment(s, living)),
                    mayor: self.mayor(),
                    lovers: self.lovers_seen_by(s),
                    theft: self.theft_seen_by(s),
                    receives: self.received_by(s),
                    audience: self.audience_of(s),
                    sees_countdown: self.sees_countdown(s),
                    may_end_debate: self.announcing().is_none()
                        && self.require_debate_leader(&s.token).is_ok(),
                    paused,
                    announcing: self.announcing(),
                    night: self.game.as_ref().map_or(0, |g| g.night),
                };
                (s.token.clone(), view)
            })
            .collect();
        Outputs {
            views,
            timer: self
                .game
                .as_ref()
                .filter(|_| !paused)
                .and_then(|g| g.timer(&self.timers)),
            grace_periods: self.grace_periods(),
            paused,
        }
    }

    /// The Grace periods running: only in the Lobby.
    fn grace_periods(&self) -> Vec<GracePeriod> {
        if self.game.is_some() {
            return Vec::new();
        }
        self.seats
            .iter()
            .filter_map(|s| {
                Some(GracePeriod {
                    disconnection: s.disconnection?,
                    player: s.id,
                    seconds: self.grace_period,
                })
            })
            .collect()
    }
}
