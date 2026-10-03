//! Reconnecting mid-Game: a Player whose phone dropped keeps their seat, gets
//! everything back when they return, and the Game never waits for them.
//! Scripted only through the engine's public commands and outputs.

use std::collections::BTreeSet;

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerSummary, PlayerView, Randomness,
    Role, RoleCounts, SeatToken, Timer,
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

/// Two Werewolves, a Seer, a Witch and three Villagers.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 1,
    witch: 1,
    hunter: 0,
    villager: 3,
};

const NAMES: [&str; 7] = ["P1", "P2", "P3", "P4", "P5", "P6", "P7"];

/// A running Game of 7 Players, P1 (the Host) to P7.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
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
        let outputs = engine.handle(&seat("P1"), Command::Start).unwrap();
        let players = NAMES
            .into_iter()
            .map(|name| {
                let view = outputs.view_for(&seat(name)).unwrap();
                (name.to_string(), view.role.as_ref().unwrap().role)
            })
            .collect();
        Self {
            engine,
            outputs,
            players,
        }
    }

    fn with_role(&self, role: Role) -> Vec<String> {
        self.players
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn one(&self, role: Role) -> String {
        self.with_role(role)[0].clone()
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

    fn id(&self, name: &str) -> PlayerId {
        self.view(name).you
    }

    fn others(&self, name: &str) -> BTreeSet<PlayerId> {
        NAMES
            .iter()
            .filter(|&&n| n != name)
            .map(|n| self.id(n))
            .collect()
    }

    /// `of` as `viewer` sees them in the list of Players.
    fn summary(&self, viewer: &str, of: &str) -> &PlayerSummary {
        let id = self.id(of);
        self.view(viewer)
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
    }

    fn connected(&self, viewer: &str, of: &str) -> bool {
        self.summary(viewer, of).connected
    }

    fn alive(&self, name: &str) -> bool {
        self.summary("P1", name).alive
    }

    fn timer(&self) -> Timer {
        self.outputs.timer().expect("the current Moment is timed")
    }

    /// The server's timer for the current Moment fires.
    fn time_runs_out(&mut self) {
        let timer = self.timer();
        self.outputs = self
            .engine
            .deadline_passed(timer.moment)
            .expect("the deadline is for the current Moment");
    }

    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn until(&mut self, reached: impl Fn(&Moment) -> bool) {
        while !reached(self.moment("P1")) {
            self.time_runs_out();
        }
    }

    /// From the start, the Werewolves kill `victim` on the first Night, and
    /// the Game runs on to the Election.
    fn kill_on_the_first_night(&mut self, victim: &str) {
        self.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        let victim = self.id(victim);
        for werewolf in self.with_role(Role::Werewolf) {
            self.act(&werewolf, Command::PickVictim { victim });
        }
        self.until(|m| matches!(m, Moment::Election { .. }));
    }
}

#[test]
fn mid_game_everyone_sees_who_is_disconnected() {
    let mut table = Table::start();

    table.act("P3", Command::Disconnect);

    for name in NAMES {
        if name != "P3" {
            assert!(!table.connected(name, "P3"), "{name}");
        }
    }
}

#[test]
fn reopening_the_link_mid_game_restores_the_seat_role_and_current_moment() {
    let mut table = Table::start();
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let werewolf = table.one(Role::Werewolf);
    let before = table.view(&werewolf).clone();
    table.act(&werewolf, Command::Disconnect);

    table.act(&werewolf, join(&werewolf));

    let after = table.view(&werewolf);
    assert_eq!(after.you, before.you);
    assert_eq!(after.role, before.role);
    assert!(matches!(
        after.moment,
        Some(Moment::WerewolvesTurn { picks: Some(_) })
    ));
    assert_eq!(after.receives, before.receives);
    assert_eq!(after.audience, before.audience);
    assert!(table.connected("P1", &werewolf));
}

#[test]
fn a_player_eliminated_while_away_comes_back_as_a_spectator() {
    let mut table = Table::start();
    let victim = table.one(Role::Villager);
    table.act(&victim, Command::Disconnect);
    table.kill_on_the_first_night(&victim);

    table.act(&victim, join(&victim));

    let view = table.view(&victim);
    assert!(view.spectating);
    assert!(table.connected("P1", &victim));
    assert!(!table.alive(&victim));
    // Spectators see everyone while no living Player sees them.
    assert_eq!(
        view.receives.iter().copied().collect::<BTreeSet<_>>(),
        table.others(&victim)
    );
    assert!(view.audience.is_empty());
}

#[test]
fn a_disconnected_seers_turn_runs_out_without_her() {
    let mut table = Table::start();
    let seer = table.one(Role::Seer);
    table.act(&seer, Command::Disconnect);
    table.until(|m| matches!(m, Moment::SeersTurn { .. }));

    table.time_runs_out();

    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}

#[test]
fn a_disconnected_werewolf_does_not_hold_up_the_kill() {
    let mut table = Table::start();
    let [away, present] = <[String; 2]>::try_from(table.with_role(Role::Werewolf)).unwrap();
    let victim = table.one(Role::Villager);
    table.act(&away, Command::Disconnect);
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));

    let victim_id = table.id(&victim);
    table.act(&present, Command::PickVictim { victim: victim_id });
    table.until(|m| matches!(m, Moment::Dawn { .. }));

    assert!(matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim_id]));
}

#[test]
fn a_disconnected_voter_counts_as_abstaining() {
    let mut table = Table::start();
    let away = table.one(Role::Witch);
    let designated = table.one(Role::Seer);
    table.until(|m| matches!(m, Moment::Vote { .. }));
    table.act(&away, Command::Disconnect);

    let designated_id = table.id(&designated);
    for name in NAMES {
        if name != away && table.alive(name) {
            table.act(
                name,
                Command::Vote {
                    designated: Some(designated_id),
                },
            );
        }
    }
    table.until(|m| !matches!(m, Moment::Vote { .. }));

    let Moment::VoteResult {
        ballots,
        eliminated,
    } = table.moment("P1")
    else {
        panic!("the Vote ended without a result: {:?}", table.moment("P1"));
    };
    assert_eq!(*eliminated, Some(designated_id));
    assert!(ballots.iter().all(|b| b.voter != table.id(&away)));
}
