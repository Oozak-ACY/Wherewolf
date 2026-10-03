//! The Mayor: the Election, the tie-break and the succession, scripted only
//! through the engine's public commands and outputs.

use wherewolf_engine::{
    Ballot, Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness,
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

/// Two Werewolves, a Hunter and four Villagers.
const WITH_HUNTER: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 0,
    hunter: 1,
    villager: 4,
};

/// A running Game with one Player per Role card, P1 (the Host) onwards.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        Self::start_with(WITH_HUNTER)
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

    /// During the Vote, every living Player votes against `designated`.
    fn village_eliminates(&mut self, designated: &str) {
        let designated = Some(self.id(designated));
        for voter in self.living() {
            self.act(&voter, Command::Vote { designated });
        }
    }

    fn shoot(&self, player: &str) -> Command {
        Command::Shoot {
            player: self.id(player),
        }
    }

    /// The Mayor every Player sees.
    fn mayor(&self) -> Option<PlayerId> {
        let everyone = self.everyone();
        let mayor = self.view(&everyone[0]).mayor;
        for name in &everyone {
            assert_eq!(self.view(name).mayor, mayor, "{name} sees another Mayor");
        }
        mayor
    }

    fn living(&self) -> Vec<String> {
        self.everyone()
            .into_iter()
            .filter(|p| self.alive(p))
            .collect()
    }

    /// From the first Night's Werewolves' Turn, the Werewolves kill
    /// `victim` and the first Day runs up to the Election.
    fn first_day_until_election(&mut self, victim: &str) {
        self.werewolves_kill(victim);
        self.time_runs_out();
        self.time_runs_out();
    }

    /// During the Election, every living Player votes for `candidate`.
    fn everyone_elects(&mut self, candidate: &str) {
        let candidate = self.id(candidate);
        for voter in self.living() {
            self.act(&voter, Command::Elect { candidate });
        }
    }

    /// From the first Night's Werewolves' Turn: the Werewolves kill a
    /// Villager, `mayor` is elected, and the first Vote begins.
    fn first_vote_with_mayor(&mut self, mayor: &str) {
        self.first_day_until_election(&self.villager(0));
        self.everyone_elects(mayor);
        self.time_runs_out();
    }

    /// During the Vote, the living split evenly between `first` and
    /// `second`, in seat order.
    fn tied_vote(&mut self, first: &str, second: &str) {
        let living = self.living();
        let (half, rest) = living.split_at(living.len() / 2);
        for (voters, designated) in [(half, first), (rest, second)] {
            let designated = Some(self.id(designated));
            for voter in voters {
                self.act(voter, Command::Vote { designated });
            }
        }
    }

    fn break_tie(&self, player: &str) -> Command {
        Command::BreakTie {
            player: self.id(player),
        }
    }
}
#[test]
fn the_living_elect_the_mayor_between_the_first_discussion_and_the_vote() {
    let mut table = Table::start();
    let candidate = table.villager(1);
    table.werewolves_kill(&table.villager(0));
    table.time_runs_out();
    assert_eq!(table.moment_for_all(), Moment::Discussion);
    assert_eq!(table.mayor(), None);

    table.time_runs_out();

    assert!(matches!(table.moment(&candidate), Moment::Election { .. }));
    table.everyone_elects(&candidate);
    let Moment::ElectionResult {
        ballots,
        mayor,
        by_lot,
    } = table.moment_for_all()
    else {
        panic!("the Election should be over");
    };
    assert_eq!(mayor, table.id(&candidate));
    assert!(!by_lot);
    assert_eq!(ballots.len(), table.living().len());
    assert!(ballots.contains(&Ballot {
        voter: table.id(&candidate),
        designated: Some(table.id(&candidate)),
    }));
    assert_eq!(table.mayor(), Some(table.id(&candidate)));

    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Vote { .. }));
}

#[test]
fn a_tied_election_is_drawn_by_lot_among_the_tied() {
    let mut table = Table::start();
    table.first_day_until_election(&table.villager(0));
    let (first, second) = (table.villager(1), table.villager(2));
    let living = table.living();
    let (half, rest) = living.split_at(living.len() / 2);
    for (voters, candidate) in [(half, &first), (rest, &second)] {
        let candidate = table.id(candidate);
        for voter in voters {
            table.act(voter, Command::Elect { candidate });
        }
    }

    let Moment::ElectionResult { mayor, by_lot, .. } = table.moment_for_all() else {
        panic!("the Election should be over");
    };
    assert!(by_lot);
    assert!([table.id(&first), table.id(&second)].contains(&mayor));
    assert_eq!(table.mayor(), Some(mayor));
}

#[test]
fn an_election_nobody_voted_in_is_drawn_by_lot_among_the_living() {
    let mut table = Table::start();
    let victim = table.villager(0);
    table.first_day_until_election(&victim);

    table.time_runs_out();

    let Moment::ElectionResult {
        ballots,
        mayor,
        by_lot,
    } = table.moment_for_all()
    else {
        panic!("the Election should be over");
    };
    assert!(ballots.is_empty());
    assert!(by_lot);
    assert_ne!(mayor, table.id(&victim));
    assert!(table.living().iter().any(|p| table.id(p) == mayor));
}

#[test]
fn only_the_first_day_has_an_election() {
    let mut table = Table::start();
    table.first_day_until_election(&table.villager(0));
    table.everyone_elects(&table.villager(1));
    table.time_runs_out();
    table.time_runs_out();
    table.time_runs_out();
    table.werewolves_kill(&table.villager(2));
    table.time_runs_out();
    assert_eq!(table.moment_for_all(), Moment::Discussion);

    table.time_runs_out();

    assert!(matches!(table.moment_for_all(), Moment::Vote { .. }));
}

#[test]
fn only_the_living_vote_for_a_living_candidate_during_the_election() {
    let mut table = Table::start();
    let (victim, voter) = (table.villager(0), table.villager(1));
    let elect_voter = Command::Elect {
        candidate: table.id(&voter),
    };
    assert_eq!(
        table.refused(&voter, elect_voter.clone()),
        Rejection::NotNow
    );
    table.first_day_until_election(&victim);

    assert_eq!(table.refused(&victim, elect_voter), Rejection::Spectating);
    let elect_victim = Command::Elect {
        candidate: table.id(&victim),
    };
    assert_eq!(table.refused(&voter, elect_victim), Rejection::NotInPlay);
}

#[test]
fn the_mayor_chooses_who_a_tied_vote_eliminates() {
    let mut table = Table::start();
    let mayor = table.villager(1);
    table.first_vote_with_mayor(&mayor);
    let (first, second) = (table.werewolf(0), table.villager(2));
    table.tied_vote(&first, &second);

    let Moment::TieBreak { ballots, tied } = table.moment_for_all() else {
        panic!("a tied Vote goes to the Mayor");
    };
    assert_eq!(ballots.len(), table.living().len());
    let mut tied_expected = vec![table.id(&first), table.id(&second)];
    let mut tied = tied;
    tied.sort();
    tied_expected.sort();
    assert_eq!(tied, tied_expected);
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);

    let choice = table.break_tie(&first);
    table.act(&mayor, choice);

    assert!(matches!(
        table.moment_for_all(),
        Moment::VoteResult { eliminated: Some(id), .. } if id == table.id(&first)
    ));
    assert!(!table.alive(&first));
    assert!(table.alive(&second));
}

#[test]
fn when_the_mayor_does_not_break_the_tie_in_time_nobody_is_eliminated() {
    let mut table = Table::start();
    table.first_vote_with_mayor(&table.villager(1));
    let (first, second) = (table.werewolf(0), table.villager(2));
    table.tied_vote(&first, &second);

    table.time_runs_out();

    assert!(matches!(
        table.moment_for_all(),
        Moment::VoteResult {
            eliminated: None,
            ..
        }
    ));
    assert!(table.alive(&first) && table.alive(&second));
}

#[test]
fn only_the_mayor_breaks_a_tie_and_only_among_the_tied() {
    let mut table = Table::start();
    let mayor = table.villager(1);
    table.first_vote_with_mayor(&mayor);
    let (first, second, other) = (table.werewolf(0), table.villager(2), table.villager(3));
    assert_eq!(
        table.refused(&mayor, table.break_tie(&first)),
        Rejection::NotNow
    );
    table.tied_vote(&first, &second);

    assert_eq!(
        table.refused(&other, table.break_tie(&first)),
        Rejection::NotYourTurn
    );
    assert_eq!(
        table.refused(&mayor, table.break_tie(&other)),
        Rejection::NotTied
    );
}

impl Table {
    fn name_successor(&self, player: &str) -> Command {
        Command::NameSuccessor {
            player: self.id(player),
        }
    }
}

#[test]
fn the_mayor_eliminated_by_the_vote_names_a_successor() {
    let mut table = Table::start();
    let (mayor, successor) = (table.villager(1), table.villager(2));
    table.first_vote_with_mayor(&mayor);
    table.village_eliminates(&mayor);
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Succession {
            mayor: table.id(&mayor)
        }
    );
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);
    assert!(!table.view(&mayor).spectating);
    let choice = table.name_successor(&successor);
    table.act(&mayor, choice);

    assert_eq!(
        table.moment_for_all(),
        Moment::SuccessionResult {
            successor: table.id(&successor),
            by_lot: false,
        }
    );
    assert_eq!(table.mayor(), Some(table.id(&successor)));
    assert!(table.view(&mayor).spectating);
    table.time_runs_out();
    assert!(matches!(
        table.moment(&mayor),
        Moment::WerewolvesTurn { .. }
    ));
}

#[test]
fn a_mayor_killed_at_night_who_names_nobody_in_time_is_succeeded_at_random() {
    let mut table = Table::start();
    let mayor = table.villager(1);
    table.first_vote_with_mayor(&mayor);
    table.time_runs_out();
    table.time_runs_out();
    table.werewolves_kill(&mayor);
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Succession { .. }));

    table.time_runs_out();

    let Moment::SuccessionResult { successor, by_lot } = table.moment_for_all() else {
        panic!("the succession should be settled");
    };
    assert!(by_lot);
    assert!(table.living().iter().any(|p| table.id(p) == successor));
    assert_eq!(table.mayor(), Some(successor));
    table.time_runs_out();
    assert_eq!(table.moment_for_all(), Moment::Discussion);
}

#[test]
fn a_hunter_who_is_mayor_shoots_first_then_names_a_successor() {
    let mut table = Table::start();
    let (hunter, shot, successor) = (table.hunter(0), table.werewolf(0), table.villager(2));
    table.first_vote_with_mayor(&hunter);
    table.village_eliminates(&hunter);
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    let shoot = table.shoot(&shot);
    table.act(&hunter, shoot);
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Succession {
            mayor: table.id(&hunter)
        }
    );
    let choice = table.name_successor(&successor);
    table.act(&hunter, choice);
    assert_eq!(table.mayor(), Some(table.id(&successor)));
}

#[test]
fn only_the_eliminated_mayor_names_a_living_successor() {
    let mut table = Table::start();
    let (mayor, victim, other) = (table.villager(1), table.villager(0), table.villager(2));
    table.first_vote_with_mayor(&mayor);
    assert_eq!(
        table.refused(&mayor, table.name_successor(&other)),
        Rejection::NotNow
    );
    table.village_eliminates(&mayor);
    table.time_runs_out();

    assert_eq!(
        table.refused(&other, table.name_successor(&other)),
        Rejection::NotYourTurn
    );
    assert_eq!(
        table.refused(&mayor, table.name_successor(&victim)),
        Rejection::NotInPlay
    );
    assert_eq!(
        table.refused(&mayor, table.name_successor(&mayor)),
        Rejection::NotInPlay
    );
}

#[test]
fn a_mayor_whose_death_decides_the_game_names_no_successor() {
    let mut table = Table::start();
    let mayor = table.villager(1);
    table.first_vote_with_mayor(&mayor);
    table.village_eliminates(&table.villager(2));
    table.time_runs_out();
    // Two Werewolves against the Hunter and one Villager once the Mayor dies.
    table.werewolves_kill(&mayor);

    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Victory {
            winner: Camp::Werewolves
        }
    );
}

#[test]
fn a_mayor_shot_by_the_hunter_names_a_successor() {
    let mut table = Table::start();
    let (mayor, hunter) = (table.werewolf(0), table.hunter(0));
    table.first_vote_with_mayor(&mayor);
    table.village_eliminates(&hunter);
    table.time_runs_out();
    let shoot = table.shoot(&mayor);
    table.act(&hunter, shoot);

    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Succession {
            mayor: table.id(&mayor)
        }
    );
}

#[test]
fn playing_again_starts_without_a_mayor() {
    let mut table = Table::start();
    let mayor = table.villager(1);
    table.first_vote_with_mayor(&mayor);
    table.village_eliminates(&table.villager(2));
    table.time_runs_out();
    table.werewolves_kill(&mayor);
    table.time_runs_out();

    table.act("P1", Command::PlayAgain);

    assert_eq!(table.mayor(), None);
}
