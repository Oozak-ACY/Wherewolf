//! The Role reveal: the Game opens with each Player reading their Role, and
//! the first Night falls only once every living Player has confirmed it.
//! Scripted only through the engine's public commands and outputs.

mod common;

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

fn join(name: &str) -> Command {
    Command::Join {
        name: name.to_string(),
    }
}

/// One Werewolf, a Seer and three Villagers.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 1,
    seer: 1,
    witch: 0,
    hunter: 0,
    cupid: 0,
    thief: 0,
    villager: 3,
};

const NAMES: [&str; 5] = ["P1", "P2", "P3", "P4", "P5"];

/// A Game of 5 Players, P1 (the Host) to P5, just started.
struct Table {
    engine: Engine,
    outputs: Outputs,
}

impl Table {
    fn start() -> Self {
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
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let mut outputs = engine.handle(&seat("P1"), Command::Start).unwrap();
        common::announce(&mut engine, &mut outputs);
        Self { engine, outputs }
    }

    fn try_act(&mut self, name: &str, command: Command) -> Result<(), Rejection> {
        self.outputs = self.engine.handle(&seat(name), command)?;
        common::announce(&mut self.engine, &mut self.outputs);
        Ok(())
    }

    fn act(&mut self, name: &str, command: Command) {
        self.try_act(name, command)
            .unwrap_or_else(|r| panic!("{name}: {r:?}"));
    }

    fn confirm(&mut self, name: &str) {
        self.act(name, Command::ConfirmRole);
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

    /// How many Players `viewer` is told are still reading, and whether
    /// `viewer` has confirmed.
    fn reveal(&self, viewer: &str) -> (u32, bool) {
        match self.moment(viewer) {
            Moment::RoleReveal {
                still_reading,
                confirmed,
            } => (*still_reading, *confirmed),
            other => panic!("{viewer} sees {other:?}, not the Role reveal"),
        }
    }

    fn night_has_fallen(&self) -> bool {
        matches!(self.moment("P1"), Moment::SeersTurn { .. })
    }
}

#[test]
fn the_game_opens_on_the_role_reveal_untimed() {
    let table = Table::start();

    for name in NAMES {
        assert_eq!(table.reveal(name), (5, false), "{name}");
        assert!(table.view(name).role.is_some(), "{name} sees their Role");
    }
    assert_eq!(table.outputs.timer(), None);
}

#[test]
fn every_phone_counts_who_is_still_reading() {
    let mut table = Table::start();

    table.confirm("P2");
    table.confirm("P4");

    assert_eq!(table.reveal("P2"), (3, true));
    assert_eq!(table.reveal("P1"), (3, false));
    assert_eq!(table.outputs.timer(), None);
}

#[test]
fn confirming_twice_changes_nothing() {
    let mut table = Table::start();

    table.confirm("P2");
    table.confirm("P2");

    assert_eq!(table.reveal("P3"), (4, false));
}

#[test]
fn the_night_falls_once_the_last_player_confirms() {
    let mut table = Table::start();

    for name in &NAMES[..4] {
        table.confirm(name);
    }
    assert!(!table.night_has_fallen());

    table.confirm("P5");

    assert!(table.night_has_fallen());
    assert!(table.outputs.timer().is_some(), "the Seer's Turn is timed");
}

#[test]
fn a_late_spectator_is_not_waited_for_and_cannot_confirm() {
    let mut table = Table::start();
    table.act("Late", join("Late"));
    assert_eq!(table.reveal("P1"), (5, false));
    assert!(table.view("Late").role.is_none());

    assert_eq!(
        table.try_act("Late", Command::ConfirmRole),
        Err(Rejection::Spectating)
    );
    for name in NAMES {
        table.confirm(name);
    }

    assert!(table.night_has_fallen());
}

#[test]
fn a_reconnecting_player_still_sees_their_role_and_can_confirm() {
    let mut table = Table::start();
    let role = table.view("P3").role.clone();
    table.act("P3", Command::Disconnect);
    for name in ["P1", "P2", "P4", "P5"] {
        table.confirm(name);
    }
    assert_eq!(table.reveal("P1"), (1, true));

    table.act("P3", join("P3"));
    assert_eq!(table.view("P3").role, role);
    assert_eq!(table.reveal("P3"), (1, false));
    table.confirm("P3");

    assert!(table.night_has_fallen());
}

#[test]
fn confirming_is_refused_once_the_reveal_is_over() {
    let mut table = Table::start();
    for name in NAMES {
        table.confirm(name);
    }

    assert_eq!(
        table.try_act("P1", Command::ConfirmRole),
        Err(Rejection::NotNow)
    );
}

#[test]
fn nobody_acts_at_night_before_the_reveal_is_over() {
    let mut table = Table::start();
    let seer = NAMES
        .into_iter()
        .find(|n| table.view(n).role.as_ref().unwrap().role == Role::Seer)
        .unwrap();
    let target = table.view(if seer == "P1" { "P2" } else { "P1" }).you;

    assert_eq!(
        table.try_act(seer, Command::Inspect { player: target }),
        Err(Rejection::NotNow)
    );
}
