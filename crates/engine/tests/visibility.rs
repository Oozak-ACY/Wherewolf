//! The visibility plan: who may see and hear whom at each Moment, and what
//! Spectators see. Driven only through the engine's public commands and outputs.

use std::collections::BTreeSet;

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Rejection, Role,
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

/// A Lobby of P1 (the Host) to P`count`.
struct Table {
    engine: Engine,
    outputs: Outputs,
    names: Vec<String>,
}

impl Table {
    fn lobby(count: usize) -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
        let names: Vec<String> = (1..=count).map(|n| format!("P{n}")).collect();
        let mut outputs = None;
        for name in &names {
            outputs = Some(engine.handle(&seat(name), join(name)).unwrap());
        }
        Self {
            engine,
            outputs: outputs.unwrap(),
            names,
        }
    }

    fn view(&self, name: &str) -> &PlayerView {
        self.outputs
            .view_for(&seat(name))
            .unwrap_or_else(|| panic!("no view for {name}"))
    }

    fn id(&self, name: &str) -> PlayerId {
        self.view(name).you
    }

    fn ids(&self, names: &[&str]) -> BTreeSet<PlayerId> {
        names.iter().map(|n| self.id(n)).collect()
    }

    /// Everyone but `name`.
    fn others(&self, name: &str) -> BTreeSet<PlayerId> {
        let everyone: Vec<&str> = self.names.iter().map(String::as_str).collect();
        let mut others = self.ids(&everyone);
        others.remove(&self.id(name));
        others
    }

    /// Who `name` may see and hear right now.
    fn receives(&self, name: &str) -> BTreeSet<PlayerId> {
        self.view(name).receives.iter().copied().collect()
    }

    /// Who may see and hear `name` right now.
    fn audience(&self, name: &str) -> BTreeSet<PlayerId> {
        self.view(name).audience.iter().copied().collect()
    }
}

#[test]
fn in_the_lobby_everyone_sees_and_hears_everyone() {
    let table = Table::lobby(5);

    for name in &table.names {
        assert_eq!(table.receives(name), table.others(name), "{name}");
        assert_eq!(table.audience(name), table.others(name), "{name}");
    }
}

impl Table {
    /// Starts a Game with `werewolves` Werewolves and Villagers for the rest.
    fn start(count: usize, werewolves: u8) -> Self {
        let mut table = Self::lobby(count);
        let mut settings = table.view("P1").settings.clone();
        settings.roles = RoleCounts {
            werewolf: werewolves,
            seer: 0,
            witch: 0,
            hunter: 0,
            villager: count as u8 - werewolves,
        };
        table.act("P1", Command::UpdateSettings { settings });
        table.act("P1", Command::Start);
        table
    }

    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn with_role(&self, role: Role) -> Vec<String> {
        self.names
            .iter()
            .filter(|n| self.view(n).role.as_ref().map(|c| c.role) == Some(role))
            .cloned()
            .collect()
    }
}

#[test]
fn during_their_turn_the_werewolves_see_and_hear_only_each_other() {
    let table = Table::start(8, 3);
    let werewolves = table.with_role(Role::Werewolf);
    let wolves: Vec<&str> = werewolves.iter().map(String::as_str).collect();

    for name in &table.names {
        if werewolves.contains(name) {
            let mut others = table.ids(&wolves);
            others.remove(&table.id(name));
            assert_eq!(table.receives(name), others, "{name}");
            assert_eq!(table.audience(name), others, "{name}");
        } else {
            assert_eq!(table.receives(name), BTreeSet::new(), "{name}");
            assert_eq!(table.audience(name), BTreeSet::new(), "{name}");
        }
    }
}

impl Table {
    fn pick(&mut self, werewolf: &str, victim: &str) {
        let victim = self.id(victim);
        self.act(werewolf, Command::PickVictim { victim });
    }

    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
        self.outputs = self.engine.deadline_passed(timer.moment).unwrap();
    }

    /// Starts a Game of 7 with 2 Werewolves, who kill a Villager on the
    /// first Night. Returns the new Spectator's name.
    fn after_a_first_death() -> (Self, String) {
        let mut table = Self::start(7, 2);
        let victim = table.with_role(Role::Villager)[0].clone();
        for werewolf in table.with_role(Role::Werewolf) {
            table.pick(&werewolf, &victim);
        }
        (table, victim)
    }

    fn living(&self) -> Vec<String> {
        let p1 = self.view("P1");
        self.names
            .iter()
            .filter(|n| p1.players.iter().any(|p| p.name == **n && p.alive))
            .cloned()
            .collect()
    }
}

#[test]
fn by_day_the_living_see_each_other_and_never_the_spectators() {
    let (mut table, spectator) = Table::after_a_first_death();
    let living = table.living();
    let living_refs: Vec<&str> = living.iter().map(String::as_str).collect();

    // Dawn, the discussion, the Vote and its result.
    for _ in 0..4 {
        for name in &living {
            let mut others = table.ids(&living_refs);
            others.remove(&table.id(name));
            assert_eq!(table.receives(name), others, "{name}");
            let mut audience = others.clone();
            audience.insert(table.id(&spectator));
            assert_eq!(table.audience(name), audience, "{name}");
        }
        assert_eq!(table.receives(&spectator), table.others(&spectator));
        assert_eq!(table.audience(&spectator), BTreeSet::new());
        table.time_runs_out();
    }
}

impl Table {
    fn skip_to_the_next_night(&mut self) {
        for _ in 0..4 {
            self.time_runs_out();
        }
    }
}

#[test]
fn spectators_watch_the_werewolves_turn_unseen() {
    let (mut table, spectator) = Table::after_a_first_death();
    table.skip_to_the_next_night();
    let werewolves = table.with_role(Role::Werewolf);
    let wolves: Vec<&str> = werewolves.iter().map(String::as_str).collect();
    let spectator_id = table.id(&spectator);

    assert_eq!(table.receives(&spectator), table.others(&spectator));
    assert_eq!(table.audience(&spectator), BTreeSet::new());
    for name in table.living() {
        let mut audience = if werewolves.contains(&name) {
            let mut others = table.ids(&wolves);
            others.remove(&table.id(&name));
            others
        } else {
            BTreeSet::new()
        };
        audience.insert(spectator_id);
        assert_eq!(table.audience(&name), audience, "{name}");
        assert!(!table.receives(&name).contains(&spectator_id), "{name}");
    }
}

#[test]
fn on_the_victory_screen_everyone_sees_everyone_again() {
    let mut table = Table::start(5, 1);
    let werewolf = table.with_role(Role::Werewolf)[0].clone();
    let villager = table.with_role(Role::Villager)[0].clone();
    table.pick(&werewolf, &villager);
    table.time_runs_out();
    table.time_runs_out();
    let werewolf_id = table.id(&werewolf);
    for voter in table.living() {
        table.act(
            &voter,
            Command::Vote {
                designated: Some(werewolf_id),
            },
        );
    }
    table.time_runs_out();

    for name in &table.names {
        assert_eq!(table.receives(name), table.others(name), "{name}");
        assert_eq!(table.audience(name), table.others(name), "{name}");
    }
}

#[test]
fn spectators_see_every_role_and_the_werewolves_picks() {
    let (mut table, spectator) = Table::after_a_first_death();
    table.skip_to_the_next_night();
    let werewolf = table.with_role(Role::Werewolf)[0].clone();
    let villager = table.with_role(Role::Villager)[1].clone();
    table.pick(&werewolf, &villager);

    let view = table.view(&spectator);
    for summary in &view.players {
        let dealt = table.view(&summary.name).role.as_ref().map(|c| c.role);
        assert_eq!(summary.revealed_role, dealt, "{}", summary.name);
    }
    match &view.moment {
        Some(Moment::WerewolvesTurn { picks }) => assert_eq!(picks.as_ref().map(Vec::len), Some(1)),
        other => panic!("expected the Werewolves' Turn, got {other:?}"),
    }
    let living_villager = table.view(&table.with_role(Role::Villager)[2]);
    let hidden = living_villager
        .players
        .iter()
        .filter(|p| p.alive && p.revealed_role.is_some())
        .count();
    assert_eq!(hidden, 0, "the living see no living Player's Role");
}

#[test]
fn a_latecomer_joins_as_a_spectator() {
    let mut table = Table::start(5, 1);

    table.act("Late", join("Late"));
    table.names.push("Late".to_string());

    let late = table.view("Late");
    let summary = late.players.iter().find(|p| p.name == "Late").unwrap();
    assert!(!summary.alive);
    assert_eq!(late.role, None);
    for p in late.players.iter().filter(|p| p.name != "Late") {
        assert!(p.revealed_role.is_some(), "Late sees {}'s Role", p.name);
    }
    assert_eq!(table.receives("Late"), table.others("Late"));
    assert_eq!(table.audience("Late"), BTreeSet::new());
    let werewolf = table.with_role(Role::Werewolf)[0].clone();
    let victim = table.id("P2");
    assert_eq!(
        table
            .engine
            .handle(&seat("Late"), Command::PickVictim { victim }),
        Err(Rejection::Spectating)
    );
    assert!(!table.receives(&werewolf).contains(&table.id("Late")));
}

#[test]
fn a_latecomer_plays_the_next_game() {
    let mut table = Table::start(5, 1);
    table.act("Late", join("Late"));
    table.names.push("Late".to_string());
    let werewolf = table.with_role(Role::Werewolf)[0].clone();
    let werewolf_id = table.id(&werewolf);
    table.time_runs_out();
    table.time_runs_out();
    table.time_runs_out();
    for voter in table.living() {
        table.act(
            &voter,
            Command::Vote {
                designated: Some(werewolf_id),
            },
        );
    }
    table.time_runs_out();

    table.act("P1", Command::PlayAgain);

    let late = table
        .view("P1")
        .players
        .iter()
        .find(|p| p.name == "Late")
        .unwrap();
    assert!(late.alive);
    assert_eq!(table.view("P1").players.len(), 6);
}

#[test]
fn during_the_seers_turn_she_and_every_living_player_see_and_hear_nobody() {
    let mut table = Table::lobby(7);
    let mut settings = table.view("P1").settings.clone();
    settings.roles = RoleCounts {
        werewolf: 2,
        seer: 1,
        witch: 0,
        hunter: 0,
        villager: 4,
    };
    table.act("P1", Command::UpdateSettings { settings });
    table.act("P1", Command::Start);
    table.act("Late", join("Late"));
    let late = table.id("Late");
    let seer = table.with_role(Role::Seer)[0].clone();
    assert!(matches!(
        table.view(&seer).moment,
        Some(Moment::SeersTurn { .. })
    ));

    for name in &table.names {
        assert_eq!(table.receives(name), BTreeSet::new(), "{name}");
        assert_eq!(table.audience(name), BTreeSet::from([late]), "{name}");
    }
    table.names.push("Late".to_string());
    assert_eq!(table.receives("Late"), table.others("Late"));
}
