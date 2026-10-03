//! The Hunter's shot and the death triggers, scripted only through the
//! engine's public commands and outputs.

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Rejection,
    Role, RoleCounts, SeatToken,
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

/// Two Werewolves, a Hunter and four Villagers.
const ONE_HUNTER: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 0,
    hunter: 1,
    villager: 4,
};

/// Two Werewolves, two Hunters and three Villagers.
const TWO_HUNTERS: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 0,
    hunter: 2,
    villager: 3,
};

/// A running Game with one Player per Role card, P1 (the Host) onwards.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        Self::start_with(ONE_HUNTER)
    }

    fn start_with(roles: RoleCounts) -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
        let names: Vec<String> = (1..=roles.total()).map(|n| format!("P{n}")).collect();
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

    fn hunter(&self, n: usize) -> String {
        self.with_role(Role::Hunter)[n].clone()
    }

    fn werewolf(&self, n: usize) -> String {
        self.with_role(Role::Werewolf)[n].clone()
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
        self.view(&self.everyone()[0])
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .alive
    }

    /// The Role `viewer` sees revealed on `name`, if any.
    fn revealed_to(&self, viewer: &str, name: &str) -> Option<Role> {
        let id = self.id(name);
        self.view(viewer)
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .revealed_role
    }

    /// Asserts every Player sees the same Moment, and returns it.
    fn moment_for_all(&self) -> Moment {
        let everyone = self.everyone();
        let moment = self.moment(&everyone[0]).clone();
        for name in &everyone {
            assert_eq!(self.moment(name), &moment, "{name} sees another Moment");
        }
        moment
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
    /// which ends their Turn (and the Night, without a Witch).
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

    /// From the Discussion, every living Player votes against `designated`.
    /// On the first Day, they elect as Mayor a Villager who survives every
    /// script here, so no succession gets in the way.
    fn village_eliminates(&mut self, designated: &str) {
        loop {
            match self.moment(&self.everyone()[0]) {
                Moment::Vote { .. } => break,
                Moment::Election { .. } => {
                    let candidate = self.id(&self.villager(3));
                    let living: Vec<String> = self
                        .everyone()
                        .into_iter()
                        .filter(|p| self.alive(p))
                        .collect();
                    for voter in living {
                        self.act(&voter, Command::Elect { candidate });
                    }
                }
                _ => self.time_runs_out(),
            }
        }
        let designated = Some(self.id(designated));
        let living: Vec<String> = self
            .everyone()
            .into_iter()
            .filter(|p| self.alive(p))
            .collect();
        for voter in living {
            self.act(&voter, Command::Vote { designated });
        }
    }

    fn shoot(&self, player: &str) -> Command {
        Command::Shoot {
            player: self.id(player),
        }
    }
}

#[test]
fn a_hunter_killed_at_night_shoots_after_the_dawn_announcement() {
    let mut table = Table::start();
    let (hunter, shot) = (table.hunter(0), table.villager(0));
    table.werewolves_kill(&hunter);
    assert!(matches!(table.moment_for_all(), Moment::Dawn { .. }));

    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    let shoot = table.shoot(&shot);
    table.act(&hunter, shoot);

    assert_eq!(
        table.moment_for_all(),
        Moment::ShotResult {
            hunter: table.id(&hunter),
            shot: Some(table.id(&shot)),
        }
    );
    assert!(!table.alive(&shot));
    for viewer in table.everyone() {
        assert_eq!(table.revealed_to(&viewer, &shot), Some(Role::Villager));
    }

    table.time_runs_out();
    assert_eq!(table.moment_for_all(), Moment::Discussion);
}

#[test]
fn a_hunter_eliminated_by_the_vote_shoots_right_after_the_result() {
    let mut table = Table::start();
    let (hunter, shot) = (table.hunter(0), table.werewolf(0));
    table.werewolves_kill(&table.villager(1));
    table.time_runs_out();
    table.village_eliminates(&hunter);
    assert!(matches!(
        table.moment_for_all(),
        Moment::VoteResult {
            eliminated: Some(_),
            ..
        }
    ));

    table.time_runs_out();
    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    let shoot = table.shoot(&shot);
    table.act(&hunter, shoot);
    assert!(!table.alive(&shot));

    table.time_runs_out();
    assert!(matches!(
        table.moment(&table.villager(0)),
        Moment::WerewolvesTurn { .. }
    ));
}

#[test]
fn when_the_hunter_does_not_shoot_in_time_the_shot_is_lost() {
    let mut table = Table::start();
    let hunter = table.hunter(0);
    table.werewolves_kill(&hunter);
    table.time_runs_out();
    let living_before = table
        .everyone()
        .into_iter()
        .filter(|p| table.alive(p))
        .count();

    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::ShotResult {
            hunter: table.id(&hunter),
            shot: None,
        }
    );
    let living_after = table
        .everyone()
        .into_iter()
        .filter(|p| table.alive(p))
        .count();
    assert_eq!(living_after, living_before);
    table.time_runs_out();
    assert_eq!(table.moment_for_all(), Moment::Discussion);
}

#[test]
fn only_the_eliminated_hunter_shoots_a_living_player_during_his_shot() {
    let mut table = Table::start();
    let (hunter, villager, victim) = (table.hunter(0), table.villager(0), table.villager(1));
    let shoot_villager = table.shoot(&villager);
    assert_eq!(
        table.refused(&hunter, shoot_villager.clone()),
        Rejection::NotNow
    );
    table.werewolves_kill(&victim);
    table.time_runs_out();
    table.village_eliminates(&hunter);
    table.time_runs_out();

    assert_eq!(
        table.refused(&villager, table.shoot(&table.werewolf(0))),
        Rejection::NotYourTurn
    );
    assert_eq!(
        table.refused(&hunter, table.shoot(&victim)),
        Rejection::NotInPlay
    );
    assert_eq!(
        table.refused(&hunter, table.shoot(&hunter)),
        Rejection::NotInPlay
    );
    table.act(&hunter, shoot_villager);
    assert_eq!(
        table.refused(&hunter, table.shoot(&table.villager(2))),
        Rejection::NotNow
    );
}

#[test]
fn a_hunter_shot_by_a_hunter_shoots_in_turn() {
    let mut table = Table::start_with(TWO_HUNTERS);
    let (first, second, werewolf) = (table.hunter(0), table.hunter(1), table.werewolf(0));
    table.werewolves_kill(&first);
    table.time_runs_out();
    let shoot = table.shoot(&second);
    table.act(&first, shoot);
    assert!(!table.alive(&second));

    table.time_runs_out();
    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&second)
        }
    );
    let shoot = table.shoot(&werewolf);
    table.act(&second, shoot);
    assert_eq!(
        table.moment_for_all(),
        Moment::ShotResult {
            hunter: table.id(&second),
            shot: Some(table.id(&werewolf)),
        }
    );

    table.time_runs_out();
    assert_eq!(table.moment_for_all(), Moment::Discussion);
}

#[test]
fn victory_is_checked_only_once_every_trigger_is_resolved() {
    let mut table = Table::start();
    let hunter = table.hunter(0);
    table.werewolves_kill(&table.villager(0));
    table.time_runs_out();
    table.village_eliminates(&table.villager(1));
    table.time_runs_out();
    // Two Werewolves against two Villagers once the Hunter is dead.
    table.werewolves_kill(&hunter);
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    let shoot = table.shoot(&table.werewolf(0));
    table.act(&hunter, shoot);
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), Moment::Discussion);
}

#[test]
fn a_shot_that_ends_the_game_is_announced_before_the_victory() {
    let mut table = Table::start();
    let hunter = table.hunter(0);
    table.werewolves_kill(&table.villager(0));
    table.time_runs_out();
    table.village_eliminates(&table.werewolf(0));
    table.time_runs_out();
    table.werewolves_kill(&hunter);
    table.time_runs_out();

    let shoot = table.shoot(&table.werewolf(1));
    table.act(&hunter, shoot);
    assert!(matches!(table.moment_for_all(), Moment::ShotResult { .. }));

    table.time_runs_out();
    assert_eq!(
        table.moment_for_all(),
        Moment::Victory {
            winner: Camp::Village
        }
    );
}

#[test]
fn the_hunter_aims_at_the_table_knowing_only_what_the_living_know() {
    let mut table = Table::start();
    let (hunter, werewolf, villager) = (table.hunter(0), table.werewolf(0), table.villager(0));
    table.werewolves_kill(&villager);
    table.time_runs_out();
    table.village_eliminates(&hunter);
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::HuntersShot { .. }));

    assert_eq!(table.revealed_to(&hunter, &werewolf), None);
    assert_eq!(table.revealed_to(&hunter, &villager), Some(Role::Villager));
    let (hunter_id, villager_id) = (table.id(&hunter), table.id(&villager));
    assert!(table.view(&werewolf).receives.contains(&hunter_id));
    assert!(table.view(&hunter).receives.contains(&table.id(&werewolf)));
    assert!(!table.view(&hunter).receives.contains(&villager_id));

    let shoot = table.shoot(&werewolf);
    table.act(&hunter, shoot);
    assert_eq!(
        table.revealed_to(&hunter, &table.werewolf(1)),
        Some(Role::Werewolf)
    );
    assert!(!table.view(&table.werewolf(1)).receives.contains(&hunter_id));
}

#[test]
fn an_eliminated_player_owed_an_action_is_not_yet_a_spectator() {
    let mut table = Table::start();
    let hunter = table.hunter(0);
    assert!(!table.view(&hunter).spectating);
    table.werewolves_kill(&hunter);
    assert!(!table.view(&hunter).spectating, "waiting for his shot");
    table.time_runs_out();
    assert!(!table.view(&hunter).spectating, "aiming");

    table.time_runs_out();

    assert!(table.view(&hunter).spectating);
}
