//! Passing: every Player called on to act can choose to do nothing, in both
//! Hidden Roles modes. Scripted only through the engine's public commands
//! and outputs.

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, Pick, PlayerId, PlayerView, Randomness, Rejection,
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

/// Two Werewolves, a Seer, a Witch, a Hunter and three Villagers.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 1,
    witch: 1,
    hunter: 1,
    villager: 3,
};

const NAMES: [&str; 8] = ["P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8"];

/// A running Game of 8 Players, P1 (the Host) to P8.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start(hidden_roles: bool) -> Self {
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
        settings.hidden_roles = hidden_roles;
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

    fn refused(&mut self, name: &str, command: Command) -> Rejection {
        self.engine
            .handle(&seat(name), command)
            .expect_err("the command should be refused")
    }

    fn until(&mut self, reached: impl Fn(&Moment) -> bool) {
        while !reached(self.moment("P1")) {
            self.time_runs_out();
        }
    }
}

// The Seer.

#[test]
fn off_the_seer_passing_ends_her_turn_at_once() {
    let mut table = Table::start(false);
    let seer = table.one(Role::Seer);

    table.act(&seer, Command::Pass);

    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}

#[test]
fn on_the_seer_passing_keeps_her_turn_running_but_ends_her_choices() {
    let mut table = Table::start(true);
    let seer = table.one(Role::Seer);
    let before = table.timer();

    table.act(&seer, Command::Pass);

    assert_eq!(table.timer(), before);
    assert!(matches!(
        table.moment(&seer),
        Moment::SeersTurn {
            inspection: None,
            passed: true
        }
    ));
    let player = table.id(&table.villager(0));
    for command in [Command::Inspect { player }, Command::Pass] {
        assert_eq!(table.refused(&seer, command), Rejection::NotNow);
    }
    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
}

#[test]
fn only_the_seer_sees_that_she_passed() {
    let mut table = Table::start(true);
    let seer = table.one(Role::Seer);

    table.act(&seer, Command::Pass);

    let villager = table.villager(0);
    assert!(matches!(
        table.moment(&villager),
        Moment::SeersTurn { passed: false, .. }
    ));
}

// The Werewolves.

impl Table {
    fn werewolves(&self) -> (String, String) {
        let werewolves = self.with_role(Role::Werewolf);
        (werewolves[0].clone(), werewolves[1].clone())
    }

    /// The Victim the Witch is told about, at the start of her Turn.
    fn witch_told(&self) -> Option<PlayerId> {
        let Moment::WitchsTurn { witch: Some(sight) } = self.moment(&self.one(Role::Witch)) else {
            panic!("not the Witch's Turn: {:?}", self.moment("P1"));
        };
        sight.victim
    }
}

#[test]
fn off_every_werewolf_passing_ends_their_turn_with_no_victim() {
    let mut table = Table::start(false);
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let (first, second) = table.werewolves();

    table.act(&first, Command::Pass);
    table.act(&second, Command::Pass);

    assert_eq!(table.witch_told(), None);
}

#[test]
fn a_werewolfs_pass_shows_as_a_pick_for_nobody() {
    let mut table = Table::start(false);
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let (first, second) = table.werewolves();

    table.act(&first, Command::Pass);

    let expected = vec![Pick {
        werewolf: table.id(&first),
        victim: None,
    }];
    assert!(matches!(
        table.moment(&second),
        Moment::WerewolvesTurn { picks: Some(picks) } if picks == &expected
    ));
}

#[test]
fn a_victim_needs_every_werewolf_even_after_one_passed() {
    let mut table = Table::start(false);
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let (first, second) = table.werewolves();
    let victim = table.id(&table.villager(0));

    table.act(&first, Command::Pass);
    table.act(&second, Command::PickVictim { victim });
    assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));

    // A Werewolf who passed may still change their mind.
    table.act(&first, Command::PickVictim { victim });
    assert_eq!(table.witch_told(), Some(victim));
}

#[test]
fn when_time_runs_out_a_pass_counts_as_a_pick_for_nobody_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        let (first, second) = table.werewolves();
        let victim = table.id(&table.villager(0));
        table.act(&first, Command::Pass);
        table.act(&second, Command::PickVictim { victim });

        table.time_runs_out();

        // One for nobody, one for the villager: a tie, so no Victim.
        assert_eq!(table.witch_told(), None, "hidden_roles: {hidden_roles}");
    }
}

#[test]
fn on_every_werewolf_passing_keeps_their_turn_running_to_no_victim() {
    let mut table = Table::start(true);
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    let (first, second) = table.werewolves();
    let before = table.timer();

    table.act(&first, Command::Pass);
    table.act(&second, Command::Pass);

    assert_eq!(table.timer(), before);
    table.time_runs_out();
    assert_eq!(table.witch_told(), None);
}

#[test]
fn during_the_werewolves_turn_only_a_werewolf_passes() {
    let mut table = Table::start(false);
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));

    let villager = table.villager(0);
    assert_eq!(
        table.refused(&villager, Command::Pass),
        Rejection::NotYourTurn
    );
}

// The Hunter and the Mayor: public, so their pass ends the Moment at once
// in both modes.

impl Table {
    /// From the start, every Werewolf picks `victim` on the first Night.
    fn pick_on_the_first_night(&mut self, victim: &str) {
        self.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        let victim = self.id(victim);
        let (first, second) = self.werewolves();
        self.act(&first, Command::PickVictim { victim });
        self.act(&second, Command::PickVictim { victim });
    }

    /// From the start, a first Night where nobody dies, then the living
    /// elect `mayor` and the Vote begins.
    fn elect_then_vote(&mut self, mayor: &str) {
        self.until(|m| matches!(m, Moment::Election { .. }));
        let candidate = self.id(mayor);
        for name in NAMES {
            self.act(name, Command::Elect { candidate });
        }
        self.until(|m| matches!(m, Moment::Vote { .. }));
    }

    /// From the start, `mayor` is elected and the Vote ties between two
    /// Villagers: the Mayor's tie-break begins.
    fn tie_with_mayor(&mut self, mayor: &str) {
        self.elect_then_vote(mayor);
        let (first, second) = self.werewolves();
        let others = [self.one(Role::Seer), self.one(Role::Witch)];
        self.vote(&[first, second], &self.villager(0));
        self.vote(&others, &self.villager(1));
        self.time_runs_out();
        assert!(matches!(self.moment("P1"), Moment::TieBreak { .. }));
    }

    fn vote(&mut self, voters: &[String], designated: &str) {
        let designated = Some(self.id(designated));
        for voter in voters {
            self.act(voter, Command::Vote { designated });
        }
    }
}

#[test]
fn the_hunter_passing_shoots_nobody_at_once_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        let hunter = table.one(Role::Hunter);
        table.pick_on_the_first_night(&hunter);
        table.until(|m| matches!(m, Moment::HuntersShot { .. }));
        let living = NAMES.iter().filter(|n| table.alive(n)).count();

        table.act(&hunter, Command::Pass);

        assert!(
            matches!(table.moment("P1"), Moment::ShotResult { shot: None, .. }),
            "hidden_roles: {hidden_roles}"
        );
        assert_eq!(NAMES.iter().filter(|n| table.alive(n)).count(), living);
    }
}

#[test]
fn the_mayor_passing_on_a_tie_eliminates_nobody_at_once_in_both_modes() {
    for hidden_roles in [false, true] {
        let mut table = Table::start(hidden_roles);
        let mayor = table.villager(2);
        table.tie_with_mayor(&mayor);

        table.act(&mayor, Command::Pass);

        assert!(
            matches!(
                table.moment("P1"),
                Moment::VoteResult {
                    eliminated: None,
                    ..
                }
            ),
            "hidden_roles: {hidden_roles}"
        );
    }
}

#[test]
fn only_the_mayor_passes_on_a_tie() {
    let mut table = Table::start(false);
    let mayor = table.villager(2);
    table.tie_with_mayor(&mayor);

    let villager = table.villager(0);
    assert_eq!(
        table.refused(&villager, Command::Pass),
        Rejection::NotYourTurn
    );
}

#[test]
fn the_eliminated_mayor_cannot_pass_on_naming_a_successor() {
    let mut table = Table::start(false);
    let mayor = table.villager(2);
    table.elect_then_vote(&mayor);
    let everyone: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    table.vote(&everyone, &mayor);
    table.until(|m| matches!(m, Moment::Succession { .. }));

    assert_eq!(table.refused(&mayor, Command::Pass), Rejection::NotNow);
}

#[test]
fn nobody_passes_when_nobody_is_called_on_to_act() {
    let mut table = Table::start(false);
    table.until(|m| matches!(m, Moment::Discussion));

    assert_eq!(table.refused("P1", Command::Pass), Rejection::NotNow);
}
