//! Cupid's Turn and the Lovers' Turn, scripted only through the engine's public commands and outputs.

mod common;

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Rejection, Role,
    RoleCounts, SeatToken, Settings,
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

/// Two Werewolves, Cupid, a Seer, a Witch and two Villagers.
fn with_cupid() -> RoleCounts {
    RoleCounts {
        werewolf: 2,
        seer: 1,
        witch: 1,
        hunter: 0,
        cupid: 1,
        thief: 0,
        little_girl: 0,
        assassin: 0,
        villager: 2,
    }
}

/// A Lobby of 7 Players, P1 (the Host) to P7, not started yet.
fn lobby(seed: u64) -> (Engine, Settings) {
    let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(seed));
    let mut outputs = None;
    for n in 1..=7 {
        let name = format!("P{n}");
        outputs = Some(engine.handle(&seat(&name), join(&name)).unwrap());
    }
    let settings = outputs
        .unwrap()
        .view_for(&seat("P1"))
        .unwrap()
        .settings
        .clone();
    (engine, settings)
}

/// A running Game of 7 Players, on the first Night.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        Self::start_with(11, false)
    }

    fn start_with(seed: u64, hidden_roles: bool) -> Self {
        let (mut engine, mut settings) = lobby(seed);
        settings.roles = with_cupid();
        settings.hidden_roles = hidden_roles;
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let mut started = engine.handle(&seat("P1"), Command::Start).unwrap();
        common::announce(&mut engine, &mut started);
        let names: Vec<String> = (1..=7).map(|n| format!("P{n}")).collect();
        // Everyone confirms their Role: the first Night falls.
        let mut outputs = None;
        for name in &names {
            outputs = Some(engine.handle(&seat(name), Command::ConfirmRole).unwrap());
        }
        let mut outputs = outputs.unwrap();
        common::announce(&mut engine, &mut outputs);
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

    fn cupid(&self) -> String {
        self.with_role(Role::Cupid)[0].clone()
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

    fn ids(&self, names: &[&str]) -> Vec<PlayerId> {
        names.iter().map(|name| self.id(name)).collect()
    }

    /// The server's timer for the current Moment fires.
    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
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

    /// Cupid names `lovers`.
    fn choose_lovers(&mut self, lovers: [&str; 2]) {
        let cupid = self.cupid();
        let lovers = self.ids(&lovers);
        self.act(&cupid, Command::ChooseLovers { lovers });
    }

    /// Two Players other than Cupid, neither a Werewolf.
    fn two_villagers(&self) -> [String; 2] {
        let villagers = self.with_role(Role::Villager);
        [villagers[0].clone(), villagers[1].clone()]
    }

    /// The Lovers, as `name` sees them.
    fn lovers_seen_by(&self, name: &str) -> Vec<PlayerId> {
        self.view(name).lovers.clone()
    }

    /// Lets the time run out until the next Night's first Turn, nobody
    /// having died: the Werewolves pick nobody and the village votes nobody out.
    fn until_the_next_night(&mut self) {
        loop {
            self.time_runs_out();
            if matches!(self.moment("P1"), Moment::VoteResult { .. }) {
                self.time_runs_out();
                return;
            }
        }
    }
}

#[test]
fn the_host_can_put_one_cupid_in_the_game_but_not_two() {
    let (mut engine, mut settings) = lobby(11);

    settings.roles = with_cupid();
    let outputs = engine
        .handle(
            &seat("P1"),
            Command::UpdateSettings {
                settings: settings.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        outputs.view_for(&seat("P1")).unwrap().settings.roles.cupid,
        1
    );
    assert_eq!(
        outputs.view_for(&seat("P1")).unwrap().start_blocked_by,
        None
    );

    settings.roles.cupid = 2;
    settings.roles.villager = 1;
    assert_eq!(
        engine.handle(&seat("P1"), Command::UpdateSettings { settings }),
        Err(Rejection::InvalidSettings)
    );
}

#[test]
fn cupid_counts_toward_the_one_role_per_player_rule() {
    let (mut engine, mut settings) = lobby(11);
    settings.roles = RoleCounts {
        villager: 3,
        ..with_cupid()
    };

    let outputs = engine
        .handle(&seat("P1"), Command::UpdateSettings { settings })
        .unwrap();

    assert_eq!(
        outputs.view_for(&seat("P1")).unwrap().start_blocked_by,
        Some(Rejection::RoleCountMismatch)
    );
}

#[test]
fn the_narrator_suggests_no_cupid() {
    for players in 5..=12 {
        assert_eq!(RoleCounts::suggested(players).cupid, 0, "{players} Players");
    }
}

#[test]
fn cupid_is_in_the_village_camp() {
    assert_eq!(Role::Cupid.camp(), wherewolf_engine::Camp::Village);
}

#[test]
fn the_first_night_calls_cupid_then_the_lovers_then_the_seer_the_werewolves_and_the_witch() {
    let mut table = Table::start();
    for name in table.everyone() {
        assert_eq!(table.moment(&name), &Moment::CupidsTurn, "{name}");
    }
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);

    table.choose_lovers(table.two_villagers().each_ref().map(String::as_str));
    assert!(matches!(table.moment("P1"), Moment::LoversTurn { .. }));

    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::WitchsTurn { .. }));
}

#[test]
fn without_a_seer_the_werewolves_wake_right_after_the_lovers() {
    let (mut engine, mut settings) = lobby(11);
    settings.roles = RoleCounts {
        seer: 0,
        villager: 3,
        ..with_cupid()
    };
    engine
        .handle(&seat("P1"), Command::UpdateSettings { settings })
        .unwrap();
    let mut outputs = engine.handle(&seat("P1"), Command::Start).unwrap();
    common::announce(&mut engine, &mut outputs);
    for n in 1..=7 {
        outputs = engine
            .handle(&seat(&format!("P{n}")), Command::ConfirmRole)
            .unwrap();
    }
    common::announce(&mut engine, &mut outputs);
    let mut table = Table {
        engine,
        outputs,
        players: Vec::new(),
    };
    assert_eq!(table.moment("P1"), &Moment::CupidsTurn);

    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::LoversTurn { .. }));
    table.time_runs_out();

    assert!(
        matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }),
        "{:?}",
        table.moment("P1")
    );
}

#[test]
fn from_the_second_night_on_neither_cupid_nor_the_lovers_wake() {
    let mut table = Table::start();
    table.choose_lovers(table.two_villagers().each_ref().map(String::as_str));

    table.until_the_next_night();

    assert!(
        matches!(table.moment("P1"), Moment::SeersTurn { .. }),
        "{:?}",
        table.moment("P1")
    );
}

#[test]
fn cupid_may_name_themself_as_a_lover() {
    let mut table = Table::start();
    let cupid = table.cupid();
    let other = table.two_villagers()[0].clone();

    table.choose_lovers([&cupid, &other]);

    let mut lovers = table.ids(&[&cupid, &other]);
    lovers.sort();
    assert_eq!(table.lovers_seen_by(&cupid), lovers);
}

#[test]
fn cupid_must_name_two_different_players() {
    let mut table = Table::start();
    let cupid = table.cupid();
    let [a, b] = table.two_villagers();

    let one = Command::ChooseLovers {
        lovers: table.ids(&[&a]),
    };
    assert_eq!(table.refused(&cupid, one), Rejection::NotTwoPlayers);
    let twice = Command::ChooseLovers {
        lovers: table.ids(&[&a, &a]),
    };
    assert_eq!(table.refused(&cupid, twice), Rejection::NotTwoPlayers);
    let three = Command::ChooseLovers {
        lovers: table.ids(&[&a, &b, &cupid]),
    };
    assert_eq!(table.refused(&cupid, three), Rejection::NotTwoPlayers);
}

#[test]
fn cupid_must_name_players_of_this_game() {
    let mut table = Table::start();
    let cupid = table.cupid();
    let a = table.two_villagers()[0].clone();
    let stranger = table.engine.handle(&seat("P8"), join("P8")).unwrap();
    let stranger = stranger.view_for(&seat("P8")).unwrap().you;

    let command = Command::ChooseLovers {
        lovers: vec![table.id(&a), stranger],
    };

    assert_eq!(table.refused(&cupid, command), Rejection::NotInPlay);
}

#[test]
fn only_cupid_names_the_lovers_and_only_during_their_turn() {
    let mut table = Table::start();
    let [a, b] = table.two_villagers();
    let command = Command::ChooseLovers {
        lovers: table.ids(&[&a, &b]),
    };
    assert_eq!(table.refused(&a, command.clone()), Rejection::NotYourTurn);

    table.choose_lovers([&a, &b]);

    let cupid = table.cupid();
    assert_eq!(table.refused(&cupid, command), Rejection::NotNow);
}

#[test]
fn if_cupid_does_not_choose_in_time_two_different_players_are_drawn() {
    for seed in [3, 11, 42, 97, 1234] {
        let mut table = Table::start_with(seed, false);

        table.time_runs_out();

        assert!(matches!(table.moment("P1"), Moment::LoversTurn { .. }));
        let lovers: Vec<String> = table
            .everyone()
            .into_iter()
            .filter(|name| !table.lovers_seen_by(name).is_empty())
            .collect();
        assert_eq!(lovers.len(), 2, "seed {seed}: {lovers:?}");
        let mut ids = table.ids(&[&lovers[0], &lovers[1]]);
        ids.sort();
        assert_eq!(table.lovers_seen_by(&lovers[0]), ids, "seed {seed}");
    }
}

#[test]
fn each_lover_sees_the_bond_and_confirms_and_the_turn_ends_once_both_have() {
    let mut table = Table::start();
    let [a, b] = table.two_villagers();
    table.choose_lovers([&a, &b]);
    assert_eq!(table.moment(&a), &Moment::LoversTurn { confirmed: false });
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);

    table.act(&a, Command::ConfirmPartner);
    assert_eq!(table.moment(&a), &Moment::LoversTurn { confirmed: true });
    assert_eq!(table.moment(&b), &Moment::LoversTurn { confirmed: false });
    // Confirming again changes nothing.
    table.act(&a, Command::ConfirmPartner);
    assert!(matches!(table.moment("P1"), Moment::LoversTurn { .. }));

    table.act(&b, Command::ConfirmPartner);
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
}

#[test]
fn only_the_lovers_confirm_and_only_during_their_turn() {
    let mut table = Table::start();
    let [a, b] = table.two_villagers();
    assert_eq!(
        table.refused(&a, Command::ConfirmPartner),
        Rejection::NotNow
    );
    table.choose_lovers([&a, &b]);

    let cupid = table.cupid();
    assert_eq!(
        table.refused(&cupid, Command::ConfirmPartner),
        Rejection::NotYourTurn
    );
}

#[test]
fn the_lovers_turn_ends_when_its_time_runs_out() {
    let mut table = Table::start();
    let [a, b] = table.two_villagers();
    table.choose_lovers([&a, &b]);
    table.act(&a, Command::ConfirmPartner);

    table.time_runs_out();

    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
}

#[test]
fn only_the_lovers_and_the_spectators_know_who_the_lovers_are() {
    let mut table = Table::start();
    let [a, b] = table.two_villagers();
    // Someone arriving after the start watches as a Spectator.
    table.engine.handle(&seat("P8"), join("P8")).unwrap();

    table.choose_lovers([&a, &b]);
    table.time_runs_out();

    let mut lovers = table.ids(&[&a, &b]);
    lovers.sort();
    assert_eq!(table.lovers_seen_by(&a), lovers);
    assert_eq!(table.lovers_seen_by(&b), lovers);
    assert_eq!(table.lovers_seen_by("P8"), lovers);
    for name in table.everyone().iter().filter(|n| **n != a && **n != b) {
        assert_eq!(table.lovers_seen_by(name), Vec::new(), "{name}");
    }
}

#[test]
fn nobody_knows_any_lovers_before_cupid_names_them() {
    let table = Table::start();

    for name in table.everyone() {
        assert_eq!(table.lovers_seen_by(&name), Vec::new(), "{name}");
    }
}

#[test]
fn cupid_acts_alone_and_the_lovers_wake_together() {
    let mut table = Table::start();
    let cupid = table.cupid();
    for name in table.everyone() {
        assert!(!table.view(&name).receives_anyone(), "{name} sees someone");
    }
    assert!(table.view(&cupid).sees_countdown);

    let [a, b] = table.two_villagers();
    table.choose_lovers([&a, &b]);

    assert_eq!(table.view(&a).receives, vec![table.id(&b)]);
    assert_eq!(table.view(&b).receives, vec![table.id(&a)]);
    for name in table.everyone().iter().filter(|n| **n != a && **n != b) {
        assert!(!table.view(name).receives_anyone(), "{name} sees someone");
    }
}

#[test]
fn with_hidden_roles_on_the_lovers_turn_is_not_feigned_and_only_they_see_its_countdown() {
    let mut table = Table::start_with(11, true);
    let cupid = table.cupid();
    assert!(table.view(&cupid).sees_countdown);
    let [a, b] = table.two_villagers();

    table.choose_lovers([&a, &b]);
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);
    assert!(table.view(&a).sees_countdown);
    assert!(!table.view(&cupid).sees_countdown);

    table.act(&a, Command::ConfirmPartner);
    table.act(&b, Command::ConfirmPartner);
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
}
