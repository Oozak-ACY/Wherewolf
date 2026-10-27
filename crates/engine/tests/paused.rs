//! Paused: while the Host (whose phone is the Table device) is Disconnected
//! during a Game, nothing runs and nothing changes, until they reclaim their
//! seat and the Game resumes on the same Moment. Scripted only through the
//! engine's public commands and outputs.

mod common;

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, PlayerView, Randomness, Rejection, Role,
    RoleCounts, SeatToken, Timer,
};

/// Deterministic randomness (xorshift), so every deal can be replayed.
struct Seeded(u64);

impl Randomness for Seeded {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn seat(name: &str) -> SeatToken {
    SeatToken::new(format!("token-{name}"))
}

fn join(name: &str) -> Command {
    Command::Join {
        name: name.to_string(),
    }
}

/// One Werewolf and four Villagers: the Village wins on the first Vote that
/// eliminates the Werewolf.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 1,
    seer: 0,
    witch: 0,
    hunter: 0,
    cupid: 0,
    thief: 0,
    little_girl: 0,
    villager: 4,
};

const NAMES: [&str; 5] = ["P1", "P2", "P3", "P4", "P5"];

/// P1 is the Host.
struct Table {
    engine: Engine,
    outputs: Outputs,
}

impl Table {
    /// A Lobby of 5, Settings set, not started.
    fn lobby() -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
        let mut outputs = None;
        for name in NAMES {
            outputs = Some(engine.handle(&seat(name), join(name)).unwrap());
        }
        let mut settings = outputs
            .unwrap()
            .view_for(&seat("P1"))
            .unwrap()
            .settings
            .clone();
        settings.roles = ROLES;
        let outputs = engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        Self { engine, outputs }
    }

    /// A Game on its Role reveal.
    fn revealing() -> Self {
        let mut table = Self::lobby();
        table.act("P1", Command::Start);
        table
    }

    /// A Game on the first Night's Werewolves' Turn.
    fn night() -> Self {
        let mut table = Self::revealing();
        for name in NAMES {
            table.act(name, Command::ConfirmRole);
        }
        assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
        table
    }

    fn werewolf(&self) -> &'static str {
        NAMES
            .into_iter()
            .find(|n| self.view(n).role.as_ref().unwrap().role == Role::Werewolf)
            .unwrap()
    }

    /// A living Villager other than the Host.
    fn villager(&self) -> &'static str {
        NAMES
            .into_iter()
            .find(|&n| n != "P1" && n != self.werewolf())
            .unwrap()
    }

    fn view(&self, name: &str) -> &PlayerView {
        self.outputs
            .view_for(&seat(name))
            .unwrap_or_else(|| panic!("no view for {name}"))
    }

    fn moment(&self, name: &str) -> &Moment {
        self.view(name)
            .moment
            .as_ref()
            .expect("the Game is running")
    }

    fn timer(&self) -> Timer {
        self.outputs.timer().expect("the current Moment is timed")
    }

    fn time_runs_out(&mut self) {
        let timer = self.timer();
        self.outputs = self
            .engine
            .deadline_passed(timer.moment)
            .expect("the deadline is for the current Moment");
        common::announce(&mut self.engine, &mut self.outputs);
    }

    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
        common::announce(&mut self.engine, &mut self.outputs);
    }

    fn refused(&mut self, name: &str, command: Command) -> Rejection {
        self.engine
            .handle(&seat(name), command)
            .expect_err("the command should be refused")
    }

    fn paused(&self) -> bool {
        let paused = self.view("P1").paused;
        for name in NAMES {
            assert_eq!(self.view(name).paused, paused, "{name}");
        }
        paused
    }

    /// The Village wins: the Werewolf is voted out on the first Day.
    fn play_to_victory(&mut self) {
        while !matches!(self.moment("P1"), Moment::Vote { .. }) {
            self.time_runs_out();
        }
        let werewolf = self.werewolf();
        let designated = Some(self.view(werewolf).you);
        for name in NAMES {
            if self
                .view(name)
                .players
                .iter()
                .any(|p| p.id == self.view(name).you && p.alive)
            {
                self.act(name, Command::Vote { designated });
            }
        }
        while !matches!(self.moment("P1"), Moment::Victory { .. }) {
            self.time_runs_out();
        }
    }
}

#[test]
fn the_host_disconnecting_mid_game_pauses_it_for_everyone() {
    let mut table = Table::night();
    assert!(!table.paused());

    table.act("P1", Command::Disconnect);

    assert!(table.paused());
    assert_eq!(table.outputs.timer(), None);
}

#[test]
fn while_paused_the_deadline_is_refused() {
    let mut table = Table::night();
    let timer = table.timer();
    table.act("P1", Command::Disconnect);

    assert_eq!(
        table.engine.deadline_passed(timer.moment),
        Err(Rejection::Paused)
    );
}

#[test]
fn while_paused_every_game_action_is_refused() {
    let mut table = Table::night();
    let werewolf = table.werewolf();
    let victim = table.view(table.villager()).you;
    table.act("P1", Command::Disconnect);

    assert_eq!(
        table.refused(werewolf, Command::PickVictim { victim }),
        Rejection::Paused
    );
    assert_eq!(table.refused(werewolf, Command::Pass), Rejection::Paused);
    assert_eq!(
        table.refused(table.villager(), Command::Vote { designated: None }),
        Rejection::Paused
    );
}

#[test]
fn the_role_reveal_pauses_too() {
    let mut table = Table::revealing();
    table.act("P1", Command::Disconnect);

    assert!(table.paused());
    assert_eq!(table.refused("P2", Command::ConfirmRole), Rejection::Paused);
}

#[test]
fn others_still_come_and_go_while_paused() {
    let mut table = Table::night();
    table.act("P1", Command::Disconnect);
    let other = table.villager();

    table.act(other, Command::Disconnect);
    table.act(other, join(other));

    assert!(table.paused());
    let id = table.view(other).you;
    let summary = table
        .view("P2")
        .players
        .iter()
        .find(|p| p.id == id)
        .unwrap();
    assert!(summary.connected);
}

#[test]
fn the_host_reclaiming_their_seat_resumes_the_same_moment() {
    let mut table = Table::night();
    let before = table.timer();
    table.act("P1", Command::Disconnect);

    table.act("P1", join("P1"));

    assert!(!table.paused());
    assert_eq!(table.timer(), before);
    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
    // The Game goes on from there.
    let werewolf = table.werewolf();
    let victim = table.view(table.villager()).you;
    table.act(werewolf, Command::PickVictim { victim });
    assert!(matches!(table.moment("P1"), Moment::Dawn { .. }));
}

#[test]
fn a_non_host_disconnecting_never_pauses() {
    let mut table = Table::night();
    let werewolf = table.werewolf();
    let away = table.villager();

    table.act(away, Command::Disconnect);

    assert!(!table.paused());
    assert!(table.outputs.timer().is_some());
    let victim = table.view(away).you;
    table.act(werewolf, Command::PickVictim { victim });
    assert!(matches!(table.moment("P1"), Moment::Dawn { .. }));
}

#[test]
fn the_lobby_never_pauses() {
    let mut table = Table::lobby();

    table.act("P1", Command::Disconnect);

    assert!(!table.paused());
}

#[test]
fn a_won_game_never_pauses() {
    let mut table = Table::night();
    table.play_to_victory();

    table.act("P1", Command::Disconnect);

    assert!(!table.paused());
}
