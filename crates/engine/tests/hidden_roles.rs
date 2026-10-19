//! The Hidden Roles Setting, scripted only through the engine's public
//! commands and outputs.

mod common;

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Rejection,
    Role, RoleCounts, SeatToken, Settings, Timer, ANNOUNCEMENT_SECONDS,
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
    cupid: 0,
    villager: 3,
};

const NAMES: [&str; 7] = ["P1", "P2", "P3", "P4", "P5", "P6", "P7"];

/// A Lobby of 7 Players, P1 (the Host) to P7, and the outputs of the last join.
fn lobby() -> (Engine, Outputs) {
    let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
    let mut outputs = None;
    for name in NAMES {
        outputs = Some(engine.handle(&seat(name), join(name)).unwrap());
    }
    (engine, outputs.unwrap())
}

fn settings_seen_by(outputs: &Outputs, name: &str) -> Settings {
    outputs.view_for(&seat(name)).unwrap().settings.clone()
}

/// A running Game of 7 Players, P1 (the Host) to P7.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start(hidden_roles: bool) -> Self {
        Self::start_with(hidden_roles, |_| {})
    }

    /// Like `start`, with the Host's own changes to the Settings.
    fn start_with(hidden_roles: bool, adjust: impl FnOnce(&mut Settings)) -> Self {
        let (mut engine, outputs) = lobby();
        let mut settings = settings_seen_by(&outputs, "P1");
        settings.roles = ROLES;
        settings.hidden_roles = hidden_roles;
        adjust(&mut settings);
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let mut started = engine.handle(&seat("P1"), Command::Start).unwrap();
        common::announce(&mut engine, &mut started);
        // Everyone confirms their Role: the first Night falls.
        let mut outputs = None;
        for name in NAMES {
            outputs = Some(engine.handle(&seat(name), Command::ConfirmRole).unwrap());
        }
        let mut outputs = outputs.unwrap();
        common::announce(&mut engine, &mut outputs);
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

    fn villager(&self, n: usize) -> String {
        self.with_role(Role::Villager)[n].clone()
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

    /// The Role of `of` as `viewer` sees it in the list of Players.
    fn role_seen(&self, viewer: &str, of: &str) -> Option<Role> {
        let id = self.id(of);
        self.view(viewer)
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .revealed_role
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
        common::announce(&mut self.engine, &mut self.outputs);
    }

    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
        common::announce(&mut self.engine, &mut self.outputs);
    }

    fn until(&mut self, reached: impl Fn(&Moment) -> bool) {
        while !reached(self.moment("P1")) {
            self.time_runs_out();
        }
    }

    /// From wherever the Game is, the Werewolves kill `victim` in the next
    /// Night, then the Night runs out its time until the dawn.
    fn kill_overnight(&mut self, victim: &str) {
        self.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        self.werewolves_pick(victim);
        self.until(|m| matches!(m, Moment::Dawn { .. }));
    }

    /// During the Werewolves' Turn, every living Werewolf picks `victim`.
    fn werewolves_pick(&mut self, victim: &str) {
        let victim = self.id(victim);
        for werewolf in self.with_role(Role::Werewolf) {
            if self.alive(&werewolf) {
                self.act(&werewolf, Command::PickVictim { victim });
            }
        }
    }

    /// On the first Night, the Witch heals the Werewolves' Victim and poisons
    /// another Villager; then the first Day elects a Villager Mayor and the
    /// second Night falls.
    fn spend_both_potions(&mut self) {
        let witch = self.one(Role::Witch);
        self.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        self.werewolves_pick(&self.villager(0));
        self.act(&witch, Command::Heal);
        let player = self.id(&self.villager(1));
        self.act(&witch, Command::Poison { player });
        self.next_night(&self.villager(0));
    }

    /// From the dawn, through the Day where nobody is voted out, to the first
    /// Turn of the next Night. The first Day elects `mayor`.
    fn next_night(&mut self, mayor: &str) {
        self.until(|m| matches!(m, Moment::Election { .. }));
        let candidate = self.id(mayor);
        for (name, _) in self.players.clone() {
            if self.alive(&name) {
                self.act(&name, Command::Elect { candidate });
            }
        }
        self.until(|m| matches!(m, Moment::SeersTurn { .. } | Moment::WerewolvesTurn { .. }));
    }
}

#[test]
fn hidden_roles_is_off_by_default_and_everyone_sees_it() {
    let (_, outputs) = lobby();

    for name in NAMES {
        assert!(!settings_seen_by(&outputs, name).hidden_roles, "{name}");
    }
}

#[test]
fn the_host_turns_hidden_roles_on_and_everyone_sees_it() {
    let (mut engine, outputs) = lobby();
    let mut settings = settings_seen_by(&outputs, "P1");
    settings.hidden_roles = true;

    let outputs = engine
        .handle(&seat("P1"), Command::UpdateSettings { settings })
        .unwrap();

    for name in NAMES {
        assert!(settings_seen_by(&outputs, name).hidden_roles, "{name}");
    }
}

#[test]
fn only_the_host_turns_hidden_roles_on() {
    let (mut engine, outputs) = lobby();
    let mut settings = settings_seen_by(&outputs, "P1");
    settings.hidden_roles = true;

    let refused = engine.handle(&seat("P2"), Command::UpdateSettings { settings });

    assert_eq!(refused, Err(Rejection::NotHost));
    let outputs = engine.handle(&seat("P2"), join("P2")).unwrap();
    assert!(!settings_seen_by(&outputs, "P1").hidden_roles);
}

// Hidden Roles off.

#[test]
fn off_an_eliminated_players_role_is_revealed_to_everyone() {
    let mut table = Table::start(false);
    let victim = table.villager(0);

    table.kill_overnight(&victim);

    let witness = table.villager(1);
    assert_eq!(table.role_seen(&witness, &victim), Some(Role::Villager));
}

#[test]
fn off_a_dead_seers_turn_is_skipped() {
    let mut table = Table::start(false);
    let seer = table.one(Role::Seer);
    table.kill_overnight(&seer);

    table.next_night(&table.villager(0));

    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}

#[test]
fn off_a_dead_witchs_turn_is_skipped() {
    let mut table = Table::start(false);
    let witch = table.one(Role::Witch);
    table.kill_overnight(&witch);
    table.next_night(&table.villager(0));
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let victim = table.villager(1);

    table.werewolves_pick(&victim);

    // The Werewolves agreeing ended the Night at once.
    let victim = table.id(&victim);
    assert!(matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim]));
}

// In both modes, a living Role's Turn ends once its action is confirmed.

#[test]
fn the_seers_turn_ends_shortly_after_she_inspects_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        let seer = table.one(Role::Seer);
        let before = table.timer();
        assert_eq!(before.seconds, 30);

        let player = table.id(&table.villager(0));
        table.act(&seer, Command::Inspect { player });

        // Just long enough to read what she saw.
        let after = table.timer();
        assert_ne!(after.moment, before.moment, "hidden_roles: {hidden_roles}");
        assert_eq!(after.seconds, ANNOUNCEMENT_SECONDS);
        assert!(matches!(
            table.moment(&seer),
            Moment::SeersTurn {
                inspection: Some(_),
                ..
            }
        ));
        table.time_runs_out();
        assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
    }
}

#[test]
fn the_werewolves_turn_ends_once_they_agree_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        let victim = table.id(&table.villager(0));

        for werewolf in table.with_role(Role::Werewolf) {
            table.act(&werewolf, Command::PickVictim { victim });
        }

        let witch = table.one(Role::Witch);
        assert!(
            matches!(
                table.moment(&witch),
                Moment::WitchsTurn { witch: Some(sight) } if sight.victim == Some(victim)
            ),
            "hidden_roles: {hidden_roles}"
        );
    }
}

#[test]
fn the_witchs_turn_ends_once_she_has_no_potion_left_to_use_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        let witch = table.one(Role::Witch);
        table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        table.werewolves_pick(&table.villager(0));

        table.act(&witch, Command::Heal);
        assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
        let player = table.id(&table.villager(1));
        table.act(&witch, Command::Poison { player });

        assert!(
            matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[player]),
            "hidden_roles: {hidden_roles}"
        );
    }
}

#[test]
fn off_a_living_witch_with_her_potions_spent_takes_no_time() {
    let mut table = Table::start(false);
    table.spend_both_potions();
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let victim = table.villager(2);

    table.werewolves_pick(&victim);

    let victim = table.id(&victim);
    assert!(matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim]));
}

#[test]
fn the_witch_passing_ends_her_turn_and_the_victim_dies_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        let witch = table.one(Role::Witch);
        table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        let victim = table.villager(0);
        table.werewolves_pick(&victim);

        table.act(&witch, Command::Pass);

        let victim = table.id(&victim);
        assert!(
            matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim]),
            "hidden_roles: {hidden_roles}"
        );
    }
}

// Hidden Roles on.

#[test]
fn on_the_living_never_see_an_eliminated_players_role_but_spectators_do() {
    let mut table = Table::start(true);
    let victim = table.villager(0);

    table.kill_overnight(&victim);

    let witness = table.villager(1);
    assert_eq!(table.role_seen(&witness, &victim), None);
    assert_eq!(table.role_seen(&victim, &witness), Some(Role::Villager));
    let werewolf = table.one(Role::Werewolf);
    assert_eq!(table.role_seen(&victim, &werewolf), Some(Role::Werewolf));
}

#[test]
fn on_the_victory_screen_reveals_every_role() {
    let mut table = Table::start(true);
    let witch = table.one(Role::Witch);
    let [first, second] = <[String; 2]>::try_from(table.with_role(Role::Werewolf)).unwrap();
    let victim = table.villager(0);

    // The Witch poisons one Werewolf; the village votes out the other.
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let victim_id = table.id(&victim);
    table.act(&first, Command::PickVictim { victim: victim_id });
    table.act(&second, Command::PickVictim { victim: victim_id });
    table.until(|m| matches!(m, Moment::WitchsTurn { .. }));
    let player = table.id(&first);
    table.act(&witch, Command::Poison { player });
    table.until(|m| matches!(m, Moment::Election { .. }));
    let candidate = table.id(&witch);
    for (name, _) in table.players.clone() {
        if table.alive(&name) {
            table.act(&name, Command::Elect { candidate });
        }
    }
    table.until(|m| matches!(m, Moment::Vote { .. }));
    let designated = Some(table.id(&second));
    for (name, _) in table.players.clone() {
        if table.alive(&name) {
            table.act(&name, Command::Vote { designated });
        }
    }
    assert!(matches!(table.moment("P1"), Moment::VoteResult { .. }));
    let witness = table.villager(1);
    assert_eq!(table.role_seen(&witness, &second), None);
    assert_eq!(table.role_seen(&witness, &first), None);

    table.time_runs_out();

    assert!(matches!(
        table.moment("P1"),
        Moment::Victory {
            winner: Camp::Village
        }
    ));
    for (name, role) in table.players.clone() {
        assert_eq!(table.role_seen(&witness, &name), Some(role), "{name}");
    }
}

#[test]
fn on_a_dead_seers_turn_is_feigned() {
    let mut table = Table::start(true);
    let seer = table.one(Role::Seer);
    table.kill_overnight(&seer);

    table.next_night(&table.villager(0));

    assert!(matches!(
        table.moment("P1"),
        Moment::SeersTurn {
            inspection: None,
            ..
        }
    ));
    assert!((10..=30).contains(&table.timer().seconds));
    let player = table.id(&table.villager(0));
    assert_eq!(
        table
            .engine
            .handle(&seat(&seer), Command::Inspect { player }),
        Err(Rejection::Spectating)
    );
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}

#[test]
fn on_a_feigned_turn_lasts_a_random_time_from_the_minimum_to_its_timer() {
    let mut table = Table::start_with(true, |s| s.timers.feigned_turn_minimum = 20);
    let seer = table.one(Role::Seer);
    table.kill_overnight(&seer);
    table.next_night(&table.villager(0));

    // Night after Night, the Werewolves spare everyone.
    let mut lengths = Vec::new();
    for _ in 0..10 {
        assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
        lengths.push(table.timer().seconds);
        table.time_runs_out();
        for werewolf in table.with_role(Role::Werewolf) {
            table.act(&werewolf, Command::Pass);
        }
        table.until(|m| matches!(m, Moment::SeersTurn { .. }));
    }

    assert!(lengths.iter().all(|s| (20..=30).contains(s)), "{lengths:?}");
    assert!(lengths.iter().any(|s| *s != lengths[0]), "{lengths:?}");
}

#[test]
fn on_a_feigned_turn_never_outlasts_its_timer() {
    let mut table = Table::start_with(true, |s| s.timers.feigned_turn_minimum = 60);
    let seer = table.one(Role::Seer);
    table.kill_overnight(&seer);

    table.next_night(&table.villager(0));

    assert_eq!(table.timer().seconds, 30);
}

#[test]
fn on_a_dead_witchs_turn_is_feigned() {
    let mut table = Table::start(true);
    let witch = table.one(Role::Witch);
    table.kill_overnight(&witch);
    table.next_night(&table.villager(0));
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let victim = table.villager(1);
    let victim_id = table.id(&victim);
    table.werewolves_pick(&victim);

    assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
    assert!((10..=30).contains(&table.timer().seconds));
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim_id]));
}

#[test]
fn on_a_dead_witch_with_no_potion_left_has_her_turn_feigned() {
    let mut table = Table::start(true);
    let witch = table.one(Role::Witch);
    table.spend_both_potions();
    table.kill_overnight(&witch);
    // A Day where nobody is voted out, to the next Night.
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let victim = table.villager(2);
    let victim_id = table.id(&victim);

    table.werewolves_pick(&victim);

    assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
    assert!((10..=30).contains(&table.timer().seconds));
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim_id]));
}

#[test]
fn on_a_living_witch_with_her_potions_spent_has_her_turn_feigned() {
    let mut table = Table::start(true);
    let witch = table.one(Role::Witch);
    table.spend_both_potions();
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let victim = table.villager(2);
    let victim_id = table.id(&victim);

    table.werewolves_pick(&victim);

    assert!(matches!(
        table.moment(&witch),
        Moment::WitchsTurn { witch: Some(sight) } if sight.victim == Some(victim_id)
    ));
    assert!((10..=30).contains(&table.timer().seconds));
    // Passing changes nothing: her Turn still lasts its feigned time.
    table.act(&witch, Command::Pass);
    assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::Dawn { deaths } if deaths == &[victim_id]));
}

// The countdown of a Night Turn.

#[test]
fn off_everyone_sees_the_countdown_of_a_night_turn() {
    let table = Table::start(false);

    for (name, _) in &table.players {
        assert!(table.view(name).sees_countdown, "{name}");
    }
}

#[test]
fn on_only_the_role_acting_sees_the_countdown_of_a_night_turn() {
    let mut table = Table::start(true);
    let seer = table.one(Role::Seer);
    assert!(table.view(&seer).sees_countdown);
    assert!(!table.view(&table.villager(0)).sees_countdown);

    table.time_runs_out();

    for werewolf in table.with_role(Role::Werewolf) {
        assert!(table.view(&werewolf).sees_countdown, "{werewolf}");
    }
    assert!(!table.view(&seer).sees_countdown);
}

#[test]
fn on_nobody_living_sees_the_countdown_of_a_feigned_turn_but_spectators_do() {
    let mut table = Table::start(true);
    let seer = table.one(Role::Seer);
    table.kill_overnight(&seer);

    table.next_night(&table.villager(0));

    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
    assert!(table.view(&seer).sees_countdown);
    for (name, _) in table.players.clone() {
        if table.alive(&name) {
            assert!(!table.view(&name).sees_countdown, "{name}");
        }
    }
}

#[test]
fn on_everyone_sees_the_countdown_by_day() {
    let mut table = Table::start(true);
    table.kill_overnight(&table.villager(0));

    for (name, _) in &table.players {
        assert!(table.view(name).sees_countdown, "{name}");
    }
}

// The Witch passing.

#[test]
fn during_the_witchs_turn_only_the_witch_passes() {
    let mut table = Table::start(false);
    // The Seer's Turn: passing is hers.
    let witch = table.one(Role::Witch);
    assert_eq!(
        table.engine.handle(&seat(&witch), Command::Pass),
        Err(Rejection::NotYourTurn)
    );
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    table.werewolves_pick(&table.villager(0));

    let villager = table.villager(1);
    assert_eq!(
        table.engine.handle(&seat(&villager), Command::Pass),
        Err(Rejection::NotYourTurn)
    );
}
