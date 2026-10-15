//! The Table device: only the Host's phone gives the Narrator its voice, so
//! only it has the live lines voiced, and only while a Game is in progress. Scripted
//! only through the engine's public commands and outputs.

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, PlayerView, Randomness, Rejection, Role,
    RoleCounts, SeatToken,
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

/// One Werewolf and four Villagers: the Village wins on the first Vote that
/// eliminates the Werewolf.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 1,
    seer: 0,
    witch: 0,
    hunter: 0,
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
            let join = Command::Join {
                name: name.to_string(),
            };
            outputs = Some(engine.handle(&seat(name), join).unwrap());
        }
        let mut table = Self {
            engine,
            outputs: outputs.unwrap(),
        };
        let mut settings = table.view("P1").settings.clone();
        settings.roles = ROLES;
        table.act("P1", Command::UpdateSettings { settings });
        table
    }

    /// A Game on its Role reveal.
    fn revealing() -> Self {
        let mut table = Self::lobby();
        table.act("P1", Command::Start);
        table
    }

    fn view(&self, name: &str) -> &PlayerView {
        self.outputs.view_for(&seat(name)).unwrap()
    }

    fn moment(&self) -> &Moment {
        self.view("P1")
            .moment
            .as_ref()
            .expect("the Game is running")
    }

    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
        self.outputs = self.engine.deadline_passed(timer.moment).unwrap();
    }

    /// The Village wins: the Werewolf is voted out on the first Day.
    fn play_to_victory(&mut self) {
        for name in NAMES {
            self.act(name, Command::ConfirmRole);
        }
        while !matches!(self.moment(), Moment::Vote { .. }) {
            self.time_runs_out();
        }
        let werewolf = NAMES
            .into_iter()
            .find(|n| self.view(n).role.as_ref().unwrap().role == Role::Werewolf)
            .unwrap();
        let designated = Some(self.view(werewolf).you);
        for name in NAMES {
            let me = self.view(name).you;
            if self
                .view(name)
                .players
                .iter()
                .any(|p| p.id == me && p.alive)
            {
                self.act(name, Command::Vote { designated });
            }
        }
        while !matches!(self.moment(), Moment::Victory { .. }) {
            self.time_runs_out();
        }
    }
}

#[test]
fn the_hosts_phone_is_the_table_device_of_a_game_in_progress() {
    let table = Table::revealing();

    assert_eq!(table.engine.require_table_device(&seat("P1")), Ok(()));
}

#[test]
fn no_other_phone_is_the_table_device() {
    let table = Table::revealing();

    assert_eq!(
        table.engine.require_table_device(&seat("P2")),
        Err(Rejection::NotHost)
    );
    assert_eq!(
        table.engine.require_table_device(&seat("stranger")),
        Err(Rejection::NotSeated)
    );
}

#[test]
fn there_is_no_table_device_in_the_lobby() {
    let table = Table::lobby();

    assert_eq!(
        table.engine.require_table_device(&seat("P1")),
        Err(Rejection::NotNow)
    );
}

#[test]
fn there_is_no_table_device_once_the_game_is_won() {
    let mut table = Table::revealing();
    table.play_to_victory();

    assert_eq!(
        table.engine.require_table_device(&seat("P1")),
        Err(Rejection::NotNow)
    );
}
