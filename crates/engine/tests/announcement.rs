//! Each Moment opens only once the Narrator has announced it: until the
//! Table device says its lines are over (or the backstop passes), the Moment
//! is held, with no timer and nothing to do in it. Scripted only through the
//! engine's public commands and outputs.

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, MomentId, Outputs, PlayerView, Randomness, Rejection, Role,
    RoleCounts, SeatToken, Timer, ANNOUNCEMENT_BACKSTOP_SECONDS,
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

/// A Seer, a Werewolf and three Villagers.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 1,
    seer: 1,
    witch: 0,
    hunter: 0,
    cupid: 0,
    thief: 0,
    little_girl: 0,
    villager: 3,
};

const NAMES: [&str; 5] = ["P1", "P2", "P3", "P4", "P5"];

/// P1 is the Host: their phone is the Table device.
struct Table {
    engine: Engine,
    outputs: Outputs,
}

impl Table {
    /// The Game just started: its Role reveal is being announced.
    fn started() -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(7));
        let mut outputs = None;
        for name in NAMES {
            outputs = Some(engine.handle(&seat(name), join(name)).unwrap());
        }
        let mut table = Self {
            engine,
            outputs: outputs.unwrap(),
        };
        let mut settings = table.view("P1").settings.clone();
        settings.roles = ROLES;
        table.act("P1", Command::UpdateSettings { settings });
        table.act("P1", Command::Start);
        table
    }

    /// The first Night's Seer's Turn, being announced.
    fn nightfall() -> Self {
        let mut table = Self::started();
        table.announce();
        for name in NAMES {
            table.act(name, Command::ConfirmRole);
        }
        assert!(matches!(table.moment(), Moment::SeersTurn { .. }));
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

    /// The Moment being announced, as every phone is told.
    fn announcing(&self) -> Option<MomentId> {
        let announcing = self.view("P1").announcing;
        for name in NAMES {
            assert_eq!(self.view(name).announcing, announcing, "{name}");
        }
        announcing
    }

    fn timer(&self) -> Timer {
        self.outputs.timer().expect("the current Moment is timed")
    }

    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn refused(&mut self, name: &str, command: Command) -> Rejection {
        self.engine
            .handle(&seat(name), command)
            .expect_err("the command should be refused")
    }

    /// The Table device has said the current Moment's lines.
    fn announce(&mut self) {
        let moment = self.announcing().expect("a Moment is being announced");
        self.act("P1", Command::Announced { moment });
    }

    fn time_runs_out(&mut self) {
        let timer = self.timer();
        self.outputs = self.engine.deadline_passed(timer.moment).unwrap();
    }

    fn holder(&self, role: Role) -> &'static str {
        NAMES
            .into_iter()
            .find(|n| self.view(n).role.as_ref().unwrap().role == role)
            .unwrap()
    }
}

#[test]
fn the_game_opens_on_the_role_reveal_being_announced() {
    let mut table = Table::started();

    assert!(matches!(table.moment(), Moment::RoleReveal { .. }));
    assert!(table.announcing().is_some());
    assert_eq!(table.refused("P2", Command::ConfirmRole), Rejection::NotNow);
}

#[test]
fn a_moment_being_announced_has_only_its_backstop_and_no_countdown() {
    let table = Table::nightfall();

    assert_eq!(table.timer().seconds, ANNOUNCEMENT_BACKSTOP_SECONDS);
    for name in NAMES {
        assert!(!table.view(name).sees_countdown, "{name}");
    }
}

#[test]
fn nobody_acts_in_a_moment_being_announced() {
    let mut table = Table::nightfall();
    let seer = table.holder(Role::Seer);
    let other = table.view(table.holder(Role::Werewolf)).you;

    assert_eq!(
        table.refused(seer, Command::Inspect { player: other }),
        Rejection::NotNow
    );
    assert_eq!(table.refused(seer, Command::Pass), Rejection::NotNow);
}

#[test]
fn once_announced_the_moment_opens_with_its_full_timer() {
    let mut table = Table::nightfall();
    let held = table.timer();

    table.announce();

    assert_eq!(table.announcing(), None);
    assert!(matches!(table.moment(), Moment::SeersTurn { .. }));
    let open = table.timer();
    assert_ne!(open.moment, held.moment);
    assert_eq!(open.seconds, table.view("P1").settings.timers.seer);
    let seer = table.holder(Role::Seer);
    table.act(seer, Command::Pass);
}

#[test]
fn every_new_moment_is_announced() {
    let mut table = Table::nightfall();
    table.announce();

    table.time_runs_out();

    assert!(matches!(table.moment(), Moment::WerewolvesTurn { .. }));
    assert!(table.announcing().is_some());
}

#[test]
fn the_seers_reading_moment_is_not_announced_again() {
    let mut table = Table::nightfall();
    table.announce();
    let seer = table.holder(Role::Seer);
    let other = table.view(table.holder(Role::Werewolf)).you;

    table.act(seer, Command::Inspect { player: other });

    assert_eq!(table.announcing(), None);
}

#[test]
fn only_the_table_device_announces() {
    let mut table = Table::nightfall();
    let moment = table.announcing().unwrap();

    assert_eq!(
        table.refused("P2", Command::Announced { moment }),
        Rejection::NotHost
    );
    assert!(table.announcing().is_some());
}

#[test]
fn an_announcement_for_another_moment_opens_nothing() {
    let mut table = Table::started();
    let reveal = table.announcing().unwrap();
    table.announce();
    for name in NAMES {
        table.act(name, Command::ConfirmRole);
    }

    // The Role reveal's announcement, arriving late, while the Night's first
    // Turn is being announced.
    assert_eq!(
        table.refused("P1", Command::Announced { moment: reveal }),
        Rejection::NotNow
    );
    assert!(table.announcing().is_some());
}

#[test]
fn announcing_twice_changes_nothing() {
    let mut table = Table::nightfall();
    let moment = table.announcing().unwrap();
    table.announce();

    assert_eq!(
        table.refused("P1", Command::Announced { moment }),
        Rejection::NotNow
    );
    assert_eq!(table.announcing(), None);
}

#[test]
fn without_its_announcement_a_moment_opens_once_the_backstop_passes() {
    let mut table = Table::nightfall();

    table.time_runs_out();

    assert_eq!(table.announcing(), None);
    assert!(matches!(table.moment(), Moment::SeersTurn { .. }));
    assert_eq!(table.timer().seconds, table.view("P1").settings.timers.seer);
}

#[test]
fn the_backstop_of_an_announcement_already_made_passes_unnoticed() {
    let mut table = Table::nightfall();
    let backstop = table.timer();
    table.announce();

    assert_eq!(
        table.engine.deadline_passed(backstop.moment),
        Err(Rejection::NotNow)
    );
}

#[test]
fn paused_while_announcing_the_moment_is_still_being_announced_on_resume() {
    let mut table = Table::nightfall();
    let moment = table.announcing().unwrap();
    let backstop = table.timer();

    table.act("P1", Command::Disconnect);
    assert!(table.view("P2").paused);
    assert_eq!(table.outputs.timer(), None);

    table.act("P1", join("P1"));
    assert_eq!(table.announcing(), Some(moment));
    assert_eq!(table.timer(), backstop);
}

#[test]
fn the_victory_is_announced_too() {
    let mut table = Table::nightfall();
    table.announce();
    // Nobody acts at Night: nobody dies, and the Day comes.
    while !matches!(table.moment(), Moment::Vote { .. }) {
        if table.announcing().is_some() {
            table.announce();
        } else {
            table.time_runs_out();
        }
    }
    table.announce();
    let werewolf = table.holder(Role::Werewolf);
    let designated = Some(table.view(werewolf).you);
    for name in NAMES {
        table.act(name, Command::Vote { designated });
    }
    while !matches!(table.moment(), Moment::Victory { .. }) {
        if table.announcing().is_some() {
            table.announce();
        } else {
            table.time_runs_out();
        }
    }

    assert!(table.announcing().is_some());
    assert_eq!(table.refused("P1", Command::PlayAgain), Rejection::NotNow);
    table.announce();
    table.act("P1", Command::PlayAgain);
}

impl Table {
    /// Lets the Game run announcing and timing out each Moment until `reached`
    fn run_until(&mut self, reached: impl Fn(&Moment) -> bool) {
        for _ in 0..100 {
            if reached(self.moment()) {
                return;
            }
            if self.announcing().is_some() {
                self.announce();
            } else {
                self.time_runs_out();
            }
        }
        panic!("never reached and stuck at {:?}", self.moment());
    }

    /// Which Night it is as every phone is told
    fn night(&self) -> u32 {
        let night = self.view("P1").night;
        for name in NAMES {
            assert_eq!(self.view(name).night, night, "{name}");
        }
        night
    }
}

#[test]
fn no_night_has_fallen_during_the_role_reveal() {
    let table = Table::started();

    assert_eq!(table.night(), 0);
}

#[test]
fn every_phone_counts_the_nights_and_the_day_after_each_shares_its_number() {
    let mut table = Table::nightfall();
    assert_eq!(table.night(), 1);

    table.run_until(|m| matches!(m, Moment::Discussion));
    assert_eq!(table.night(), 1);

    table.run_until(|m| matches!(m, Moment::SeersTurn { .. }));
    assert_eq!(table.night(), 2);

    table.run_until(|m| matches!(m, Moment::Discussion));
    assert_eq!(table.night(), 2);
}
