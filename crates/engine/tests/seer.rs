//! The Seer's Turn, scripted only through the engine's public commands and outputs.

use wherewolf_engine::{
    Command, Engine, Inspection, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness,
    Rejection, Role, RoleCounts, SeatToken,
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

/// A running Game of 7 Players, P1 (the Host) to P7: two Werewolves, a Seer
/// and four Villagers.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
        let names: Vec<String> = (1..=7).map(|n| format!("P{n}")).collect();
        let mut outputs = None;
        for name in &names {
            outputs = Some(engine.handle(&seat(name), join(name)).unwrap());
        }
        let mut settings = outputs
            .unwrap()
            .view_for(&seat("P1"))
            .unwrap()
            .settings
            .clone();
        settings.roles = RoleCounts {
            werewolf: 2,
            seer: 1,
            witch: 0,
            hunter: 0,
            villager: 4,
        };
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let outputs = engine.handle(&seat("P1"), Command::Start).unwrap();
        let players = names
            .into_iter()
            .map(|name| {
                let role = outputs
                    .view_for(&seat(&name))
                    .unwrap()
                    .role
                    .as_ref()
                    .unwrap()
                    .role;
                (name, role)
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

    fn seer(&self) -> String {
        self.with_role(Role::Seer)[0].clone()
    }

    fn everyone(&self) -> Vec<String> {
        self.players.iter().map(|(name, _)| name.clone()).collect()
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

    /// The server's timer for the current Moment fires.
    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
        self.outputs = self
            .engine
            .deadline_passed(timer.moment)
            .expect("the deadline is for the current Moment");
    }
}

#[test]
fn the_night_begins_with_the_seers_turn_then_the_werewolves() {
    let mut table = Table::start();

    for name in table.everyone() {
        assert!(
            matches!(table.moment(&name), Moment::SeersTurn { .. }),
            "{name} sees {:?}",
            table.moment(&name)
        );
    }
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);

    table.time_runs_out();

    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}

impl Table {
    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn inspect(&mut self, seer: &str, player: &str) {
        let player = self.id(player);
        self.act(seer, Command::Inspect { player });
    }

    fn inspection_seen_by(&self, name: &str) -> Option<Inspection> {
        match self.moment(name) {
            Moment::SeersTurn { inspection } => *inspection,
            other => panic!("{name} sees {other:?}"),
        }
    }
}

#[test]
fn the_seer_sees_the_exact_role_of_the_player_she_inspects_and_nobody_else_does() {
    let mut table = Table::start();
    let seer = table.seer();
    let werewolf = table.with_role(Role::Werewolf)[0].clone();

    table.inspect(&seer, &werewolf);

    assert_eq!(
        table.inspection_seen_by(&seer),
        Some(Inspection {
            player: table.id(&werewolf),
            role: Role::Werewolf,
        })
    );
    for name in table.everyone().iter().filter(|n| **n != seer) {
        assert_eq!(table.inspection_seen_by(name), None, "{name}");
        assert_eq!(
            table
                .view(name)
                .players
                .iter()
                .find(|p| p.id == table.id(&werewolf))
                .unwrap()
                .revealed_role,
            None
        );
    }
}

impl Table {
    /// Asserts `command` is refused and changes nothing, and says why.
    fn refused(&mut self, name: &str, command: Command) -> Rejection {
        let before = self.engine.handle(&seat(name), join(name)).unwrap();
        let reason = self
            .engine
            .handle(&seat(name), command)
            .expect_err("the command should be refused");
        let after = self.engine.handle(&seat(name), join(name)).unwrap();
        assert_eq!(before, after, "a refused command changed the Game");
        reason
    }

    fn inspect_command(&self, player: &str) -> Command {
        Command::Inspect {
            player: self.id(player),
        }
    }

    /// The Werewolves kill `victim`, then the Day passes with nobody voted
    /// out, until the next Night.
    fn kill_overnight(&mut self, victim: &str) {
        self.time_runs_out();
        let victim_id = self.id(victim);
        for werewolf in self.with_role(Role::Werewolf) {
            self.act(&werewolf, Command::PickVictim { victim: victim_id });
        }
        loop {
            self.time_runs_out();
            if matches!(
                self.moment("P1"),
                Moment::SeersTurn { .. } | Moment::WerewolvesTurn { .. }
            ) {
                break;
            }
        }
    }
}

#[test]
fn the_seer_cannot_inspect_herself() {
    let mut table = Table::start();
    let seer = table.seer();

    let command = table.inspect_command(&seer);

    assert_eq!(table.refused(&seer, command), Rejection::Yourself);
}

#[test]
fn the_seer_cannot_inspect_a_dead_player() {
    let mut table = Table::start();
    let seer = table.seer();
    let villager = table.with_role(Role::Villager)[0].clone();
    table.kill_overnight(&villager);

    let command = table.inspect_command(&villager);

    assert_eq!(table.refused(&seer, command), Rejection::NotInPlay);
}

#[test]
fn only_the_seer_inspects_once_and_only_during_her_turn() {
    let mut table = Table::start();
    let seer = table.seer();
    let [werewolf, villager, other] = [
        table.with_role(Role::Werewolf)[0].clone(),
        table.with_role(Role::Villager)[0].clone(),
        table.with_role(Role::Villager)[1].clone(),
    ];

    let command = table.inspect_command(&other);
    assert_eq!(table.refused(&villager, command), Rejection::NotYourTurn);
    let command = Command::PickVictim {
        victim: table.id(&other),
    };
    assert_eq!(table.refused(&werewolf, command), Rejection::NotNow);

    table.inspect(&seer, &villager);
    let command = table.inspect_command(&other);
    assert_eq!(table.refused(&seer, command), Rejection::NotNow);

    table.time_runs_out();
    let command = table.inspect_command(&other);
    assert_eq!(table.refused(&seer, command), Rejection::NotNow);
}

#[test]
fn if_the_seer_does_not_choose_in_time_she_learns_nothing() {
    let mut table = Table::start();
    let seer = table.seer();

    table.time_runs_out();

    assert!(matches!(table.moment(&seer), Moment::WerewolvesTurn { .. }));
    let others = table.everyone().into_iter().filter(|n| *n != seer);
    for name in others {
        let id = table.id(&name);
        let seen = table
            .view(&seer)
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap();
        assert_eq!(seen.revealed_role, None, "{name}");
    }
}

#[test]
fn spectators_see_what_the_seer_learns() {
    let mut table = Table::start();
    let seer = table.seer();
    let [dead, inspected] = [
        table.with_role(Role::Villager)[0].clone(),
        table.with_role(Role::Villager)[1].clone(),
    ];
    table.kill_overnight(&dead);

    table.inspect(&seer, &inspected);

    assert_eq!(
        table.inspection_seen_by(&dead),
        Some(Inspection {
            player: table.id(&inspected),
            role: Role::Villager,
        })
    );
}

#[test]
fn once_the_seer_is_dead_the_night_begins_with_the_werewolves() {
    let mut table = Table::start();
    let seer = table.seer();

    table.kill_overnight(&seer);

    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}
