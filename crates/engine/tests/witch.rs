//! The Witch's Turn, scripted only through the engine's public commands and outputs.

use std::collections::BTreeSet;

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Potions, Randomness,
    Rejection, Role, RoleCounts, SeatToken, WitchSight,
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

/// Two Werewolves, a Witch and four Villagers.
const WITHOUT_SEER: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 1,
    hunter: 0,
    villager: 4,
};

/// Two Werewolves, a Seer, a Witch and three Villagers.
const WITH_SEER: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 1,
    witch: 1,
    hunter: 0,
    villager: 3,
};

/// A running Game of 7 Players, P1 (the Host) to P7.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        Self::start_with(WITHOUT_SEER)
    }

    fn start_with(roles: RoleCounts) -> Self {
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
        settings.roles = roles;
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

    fn witch(&self) -> String {
        self.with_role(Role::Witch)[0].clone()
    }

    fn villager(&self, n: usize) -> String {
        self.with_role(Role::Villager)[n].clone()
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

    fn alive(&self, name: &str) -> bool {
        let id = self.id(name);
        self.view("P1")
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .alive
    }

    /// The server's timer for the current Moment fires.
    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
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

    /// During the Werewolves' Turn, every living Werewolf picks `victim`,
    /// which ends their Turn.
    fn werewolves_kill(&mut self, victim: &str) {
        let victim = self.id(victim);
        let werewolves = self
            .with_role(Role::Werewolf)
            .into_iter()
            .filter(|w| self.alive(w));
        for werewolf in werewolves.collect::<Vec<_>>() {
            self.act(&werewolf, Command::PickVictim { victim });
        }
    }

    fn poison(&self, player: &str) -> Command {
        Command::Poison {
            player: self.id(player),
        }
    }

    /// What `name` sees of the Witch's Turn.
    fn sight_of(&self, name: &str) -> Option<WitchSight> {
        match self.moment(name) {
            Moment::WitchsTurn { witch } => *witch,
            other => panic!("{name} sees {other:?}"),
        }
    }

    fn dawn_deaths(&self) -> Vec<PlayerId> {
        match self.moment("P1") {
            Moment::Dawn { deaths } => deaths.clone(),
            other => panic!("expected the dawn, got {other:?}"),
        }
    }

    fn potions(&self, name: &str) -> Option<Potions> {
        self.view(name).role.as_ref().unwrap().potions
    }

    /// From the dawn, the Day passes with nobody voted out, until the
    /// Werewolves' Turn of the next Night.
    fn next_night(&mut self) {
        loop {
            self.time_runs_out();
            if matches!(self.moment("P1"), Moment::WerewolvesTurn { .. }) {
                break;
            }
        }
    }

    fn receives(&self, name: &str) -> BTreeSet<PlayerId> {
        self.view(name).receives.iter().copied().collect()
    }

    fn audience(&self, name: &str) -> BTreeSet<PlayerId> {
        self.view(name).audience.iter().copied().collect()
    }
}

#[test]
fn the_witchs_turn_comes_after_the_werewolves_then_dawn() {
    let mut table = Table::start_with(WITH_SEER);
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));

    table.time_runs_out();

    for name in table.everyone() {
        assert!(
            matches!(table.moment(&name), Moment::WitchsTurn { .. }),
            "{name} sees {:?}",
            table.moment(&name)
        );
    }
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);

    table.time_runs_out();

    assert!(matches!(table.moment("P1"), Moment::Dawn { .. }));
}

#[test]
fn the_werewolves_agreeing_on_a_victim_wakes_the_witch() {
    let mut table = Table::start();
    let victim = table.villager(0);

    table.werewolves_kill(&victim);

    assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
    assert!(table.alive(&victim), "the Victim dies only at dawn");
}

#[test]
fn the_witch_sees_the_victim_and_nobody_else_living_does() {
    let mut table = Table::start();
    let witch = table.witch();
    let victim = table.villager(0);

    table.werewolves_kill(&victim);

    assert_eq!(
        table.sight_of(&witch),
        Some(WitchSight {
            victim: Some(table.id(&victim)),
            healed: false,
            poisoned: None,
            passed: false,
        })
    );
    for name in table.everyone().iter().filter(|n| **n != witch) {
        assert_eq!(table.sight_of(name), None, "{name}");
    }
}

#[test]
fn the_witch_sees_when_there_is_no_victim_and_cannot_heal() {
    let mut table = Table::start();
    let witch = table.witch();

    table.time_runs_out();

    assert_eq!(
        table.sight_of(&witch),
        Some(WitchSight {
            victim: None,
            healed: false,
            poisoned: None,
            passed: false,
        })
    );
    assert_eq!(table.refused(&witch, Command::Heal), Rejection::NoVictim);
}

#[test]
fn healing_saves_the_victim() {
    let mut table = Table::start();
    let witch = table.witch();
    let victim = table.villager(0);
    table.werewolves_kill(&victim);

    table.act(&witch, Command::Heal);

    assert!(table.sight_of(&witch).unwrap().healed);
    table.time_runs_out();
    assert_eq!(table.dawn_deaths(), Vec::new());
    assert!(table.alive(&victim));
}

#[test]
fn the_witch_can_heal_herself() {
    let mut table = Table::start();
    let witch = table.witch();
    table.werewolves_kill(&witch);

    table.act(&witch, Command::Heal);
    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), Vec::new());
    assert!(table.alive(&witch));
}

#[test]
fn without_healing_the_victim_dies_at_dawn() {
    let mut table = Table::start();
    let victim = table.villager(0);
    table.werewolves_kill(&victim);

    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), vec![table.id(&victim)]);
    assert!(!table.alive(&victim));
}

#[test]
fn poison_eliminates_the_chosen_player_at_dawn() {
    let mut table = Table::start();
    let witch = table.witch();
    let [victim, poisoned] = [table.villager(0), table.villager(2)];
    table.werewolves_kill(&victim);

    let command = table.poison(&poisoned);
    table.act(&witch, command);

    assert_eq!(
        table.sight_of(&witch).unwrap().poisoned,
        Some(table.id(&poisoned))
    );
    assert!(table.alive(&poisoned), "the poisoned die only at dawn");
    table.time_runs_out();
    assert_eq!(
        table.dawn_deaths(),
        vec![table.id(&victim), table.id(&poisoned)]
    );
    assert!(!table.alive(&poisoned));
}

#[test]
fn the_witch_can_heal_and_poison_on_the_same_night() {
    let mut table = Table::start();
    let witch = table.witch();
    let [victim, poisoned] = [table.villager(0), table.villager(1)];
    table.werewolves_kill(&victim);

    table.act(&witch, Command::Heal);
    let command = table.poison(&poisoned);
    table.act(&witch, command);

    // With no potion left, her Turn ended at once.
    assert_eq!(table.dawn_deaths(), vec![table.id(&poisoned)]);
    assert!(table.alive(&victim));
}

#[test]
fn dawn_announces_deaths_in_seat_order_whatever_killed_them() {
    let mut table = Table::start();
    let witch = table.witch();
    let [first, second] = [table.villager(0), table.villager(1)];
    // The Werewolves kill the later-seated Player, the Witch poisons the earlier one.
    table.werewolves_kill(&second);

    let command = table.poison(&first);
    table.act(&witch, command);
    table.time_runs_out();

    assert_eq!(
        table.dawn_deaths(),
        vec![table.id(&first), table.id(&second)]
    );
}

#[test]
fn poisoning_the_victim_kills_them_once() {
    let mut table = Table::start();
    let witch = table.witch();
    let victim = table.villager(0);
    table.werewolves_kill(&victim);

    let command = table.poison(&victim);
    table.act(&witch, command);
    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), vec![table.id(&victim)]);
}

#[test]
fn poisoning_the_last_werewolf_wins_the_game_for_the_village() {
    let mut table = Table::start_with(RoleCounts {
        werewolf: 1,
        seer: 0,
        witch: 1,
        hunter: 0,
        villager: 5,
    });
    let witch = table.witch();
    let werewolf = table.with_role(Role::Werewolf)[0].clone();
    table.time_runs_out();

    let command = table.poison(&werewolf);
    table.act(&witch, command);
    assert_eq!(table.dawn_deaths(), vec![table.id(&werewolf)]);
    table.time_runs_out();

    assert!(matches!(
        table.moment("P1"),
        Moment::Victory {
            winner: Camp::Village
        }
    ));
}

#[test]
fn each_potion_works_once_per_game() {
    let mut table = Table::start();
    let witch = table.witch();
    let [victim, poisoned, later] = [table.villager(0), table.villager(1), table.villager(2)];
    assert_eq!(
        table.potions(&witch),
        Some(Potions {
            healing: true,
            poison: true,
        })
    );
    table.werewolves_kill(&victim);
    table.act(&witch, Command::Heal);
    assert_eq!(table.refused(&witch, Command::Heal), Rejection::PotionUsed);
    table.time_runs_out();
    table.next_night();
    table.werewolves_kill(&later);
    assert_eq!(table.refused(&witch, Command::Heal), Rejection::PotionUsed);

    let command = table.poison(&poisoned);
    table.act(&witch, command);

    assert_eq!(
        table.potions(&witch),
        Some(Potions {
            healing: false,
            poison: false,
        })
    );
    // A poison refused for having been used is covered with Hidden Roles on,
    // since with it off her Turn ends once she has no potion left.
}

#[test]
fn only_the_witch_sees_her_potions() {
    let table = Table::start();

    for name in table.everyone().iter().filter(|n| **n != table.witch()) {
        assert_eq!(table.potions(name), None, "{name}");
    }
}

#[test]
fn only_the_witch_uses_potions_and_only_during_her_turn() {
    let mut table = Table::start();
    let witch = table.witch();
    let [villager, target] = [table.villager(0), table.villager(1)];

    assert_eq!(table.refused(&witch, Command::Heal), Rejection::NotNow);
    let command = table.poison(&target);
    assert_eq!(table.refused(&witch, command), Rejection::NotNow);

    table.werewolves_kill(&target);
    assert_eq!(
        table.refused(&villager, Command::Heal),
        Rejection::NotYourTurn
    );
    let command = table.poison(&target);
    assert_eq!(table.refused(&villager, command), Rejection::NotYourTurn);
}

#[test]
fn the_witch_cannot_poison_a_dead_player() {
    let mut table = Table::start();
    let witch = table.witch();
    let dead = table.villager(0);
    table.werewolves_kill(&dead);
    table.time_runs_out();
    table.next_night();
    table.time_runs_out();

    let command = table.poison(&dead);

    assert_eq!(table.refused(&witch, command), Rejection::NotInPlay);
}

#[test]
fn during_the_witchs_turn_every_living_player_sees_and_hears_nobody() {
    let mut table = Table::start();
    let dead = table.villager(0);
    table.werewolves_kill(&dead);
    table.time_runs_out();
    table.next_night();

    table.time_runs_out();

    assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
    let spectator = BTreeSet::from([table.id(&dead)]);
    for name in table.everyone().iter().filter(|n| **n != dead) {
        assert_eq!(table.receives(name), BTreeSet::new(), "{name}");
        assert_eq!(table.audience(name), spectator, "{name}");
    }
}

#[test]
fn spectators_see_what_the_witch_sees_and_does() {
    let mut table = Table::start();
    let witch = table.witch();
    let [dead, victim] = [table.villager(0), table.villager(1)];
    table.werewolves_kill(&dead);
    table.time_runs_out();
    table.next_night();
    table.werewolves_kill(&victim);

    table.act(&witch, Command::Heal);

    assert_eq!(table.sight_of(&dead), table.sight_of(&witch));
    assert!(table.sight_of(&dead).unwrap().healed);
}

#[test]
fn once_the_witch_is_dead_the_werewolves_turn_ends_the_night() {
    let mut table = Table::start();
    let witch = table.witch();
    table.werewolves_kill(&witch);
    table.time_runs_out();
    table.next_night();
    let victim = table.villager(0);

    table.werewolves_kill(&victim);

    assert_eq!(table.dawn_deaths(), vec![table.id(&victim)]);
    let command = table.poison(&victim);
    assert_eq!(table.refused(&witch, command), Rejection::Spectating);
}
