//! Full Games, scripted only through the engine's public commands and outputs.

use wherewolf_engine::{
    Ballot, Camp, Command, Engine, LobbyCode, Moment, Outputs, Pick, PlayerId, PlayerSummary,
    PlayerView, Randomness, Rejection, Role, RoleCounts, SeatToken,
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

/// A running Game, with each Player's name and dealt Role.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    /// Starts a Game of `count` Players, P1 (the Host) to P`count`, with
    /// `werewolves` Werewolves and Villagers for the rest.
    fn start(count: usize, werewolves: u8) -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
        let names: Vec<String> = (1..=count).map(|n| format!("P{n}")).collect();
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
            werewolf: werewolves,
            seer: 0,
            witch: 0,
            hunter: 0,
            villager: count as u8 - werewolves,
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

    fn with_role(&self, role: Role) -> Vec<&str> {
        self.players
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    fn werewolves(&self) -> Vec<&str> {
        self.with_role(Role::Werewolf)
    }

    fn villagers(&self) -> Vec<&str> {
        self.with_role(Role::Villager)
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
        self.view("P1")
            .players
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} is not seated"))
            .id
    }

    fn everyone(&self) -> Vec<String> {
        self.players.iter().map(|(name, _)| name.clone()).collect()
    }
}

#[test]
fn the_game_starts_with_the_werewolves_turn() {
    let table = Table::start(5, 1);

    for name in table.everyone() {
        assert!(
            matches!(table.moment(&name), Moment::WerewolvesTurn { .. }),
            "{name} sees {:?}",
            table.moment(&name)
        );
    }
}

impl Table {
    fn act(&mut self, name: &str, command: Command) {
        self.outputs = self
            .engine
            .handle(&seat(name), command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn pick(&mut self, werewolf: &str, victim: &str) {
        let victim = self.id(victim);
        self.act(werewolf, Command::PickVictim { victim });
    }

    fn picks_seen_by(&self, name: &str) -> Option<Vec<Pick>> {
        match self.moment(name) {
            Moment::WerewolvesTurn { picks } => picks.clone(),
            other => panic!("{name} sees {other:?}"),
        }
    }
}

#[test]
fn werewolves_see_each_others_picks_live_and_nobody_else_does() {
    let mut table = Table::start(7, 2);
    let [first, second] = table.werewolves()[..] else {
        panic!("two Werewolves")
    };
    let (first, second) = (first.to_string(), second.to_string());
    let villager = table.villagers()[0].to_string();

    table.pick(&first, &villager);

    let expected = vec![Pick {
        werewolf: table.id(&first),
        victim: table.id(&villager),
    }];
    assert_eq!(table.picks_seen_by(&first), Some(expected.clone()));
    assert_eq!(table.picks_seen_by(&second), Some(expected));
    assert_eq!(table.picks_seen_by(&villager), None);
}

impl Table {
    fn summary(&self, viewer: &str, name: &str) -> &PlayerSummary {
        let id = self.id(name);
        self.view(viewer)
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
    }

    fn is_alive(&self, name: &str) -> bool {
        self.summary("P1", name).alive
    }
}

#[test]
fn a_unanimous_pick_kills_the_victim_at_dawn_and_reveals_their_role() {
    let mut table = Table::start(7, 2);
    let werewolves: Vec<String> = table.werewolves().iter().map(|w| w.to_string()).collect();
    let victim = table.villagers()[0].to_string();

    for werewolf in &werewolves {
        table.pick(werewolf, &victim);
    }

    let victim_id = table.id(&victim);
    for name in table.everyone() {
        assert_eq!(
            table.moment(&name),
            &Moment::Dawn {
                deaths: vec![victim_id]
            }
        );
        let seen = table.summary(&name, &victim);
        assert!(!seen.alive);
        assert_eq!(seen.revealed_role, Some(Role::Villager));
    }
    assert!(table.is_alive(&werewolves[0]));
    assert_eq!(table.summary("P1", &werewolves[0]).revealed_role, None);
}

impl Table {
    /// The server's timer for the current step fires.
    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current step is timed");
        self.outputs = self
            .engine
            .deadline_passed(timer.moment)
            .expect("the deadline is for the current step");
    }

    fn dawn_deaths(&self) -> Vec<PlayerId> {
        match self.moment("P1") {
            Moment::Dawn { deaths } => deaths.clone(),
            other => panic!("expected dawn, got {other:?}"),
        }
    }

    fn two_werewolves_and_two_villagers(&self) -> [String; 4] {
        let w = self.werewolves();
        let v = self.villagers();
        [w[0], w[1], v[0], v[1]].map(String::from)
    }
}

#[test]
fn the_werewolves_turn_lasts_its_timer() {
    let table = Table::start(7, 2);

    assert_eq!(table.outputs.timer().unwrap().seconds, 60);
}

#[test]
fn on_timeout_the_most_picked_player_is_the_victim() {
    let mut table = Table::start(7, 2);
    let [first, _, villager, _] = table.two_werewolves_and_two_villagers();

    table.pick(&first, &villager);
    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), vec![table.id(&villager)]);
    assert!(!table.is_alive(&villager));
}

#[test]
fn on_timeout_a_tie_means_no_victim() {
    let mut table = Table::start(7, 2);
    let [first, second, one, other] = table.two_werewolves_and_two_villagers();

    table.pick(&first, &one);
    table.pick(&second, &other);
    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), vec![]);
    assert!(table.is_alive(&one) && table.is_alive(&other));
}

#[test]
fn on_timeout_no_picks_means_no_victim() {
    let mut table = Table::start(7, 2);

    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), vec![]);
}

#[test]
fn after_dawn_the_village_discusses_then_votes() {
    let mut table = Table::start(7, 2);
    table.time_runs_out();
    assert_eq!(table.outputs.timer().unwrap().seconds, 10);

    table.time_runs_out();
    assert_eq!(table.moment("P3"), &Moment::Discussion);
    assert_eq!(table.outputs.timer().unwrap().seconds, 300);

    table.time_runs_out();
    assert!(matches!(table.moment("P3"), Moment::Vote { .. }));
    assert_eq!(table.outputs.timer().unwrap().seconds, 60);
}

impl Table {
    /// Skips a quiet Night (no Victim) and the discussion.
    fn skip_to_the_vote(&mut self) {
        while !matches!(self.moment("P1"), Moment::Vote { .. }) {
            self.time_runs_out();
        }
    }

    fn vote(&mut self, voter: &str, designated: &str) {
        let designated = Some(self.id(designated));
        self.act(voter, Command::Vote { designated });
    }

    fn abstain(&mut self, voter: &str) {
        self.act(voter, Command::Vote { designated: None });
    }

    fn vote_result(&self, viewer: &str) -> (Vec<Ballot>, Option<PlayerId>) {
        match self.moment(viewer) {
            Moment::VoteResult {
                ballots,
                eliminated,
            } => (ballots.clone(), *eliminated),
            other => panic!("expected the Vote result, got {other:?}"),
        }
    }

    fn ballot(&self, voter: &str, designated: Option<&str>) -> Ballot {
        Ballot {
            voter: self.id(voter),
            designated: designated.map(|d| self.id(d)),
        }
    }
}

#[test]
fn votes_stay_hidden_until_everyone_has_voted_then_the_most_voted_is_eliminated() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();
    let werewolf = table.werewolves()[0].to_string();
    let everyone = table.everyone();
    let (last, others) = everyone.split_last().unwrap();

    for voter in others {
        table.vote(voter, &werewolf);
    }
    let voted: Vec<PlayerId> = others.iter().map(|v| table.id(v)).collect();
    for name in &everyone {
        assert!(matches!(table.moment(name), Moment::Vote { voted: v, .. } if v == &voted));
    }

    table.vote(last, others.last().unwrap());

    let mut expected: Vec<Ballot> = others
        .iter()
        .map(|v| table.ballot(v, Some(&werewolf)))
        .collect();
    expected.push(table.ballot(last, Some(others.last().unwrap())));
    for name in &everyone {
        assert_eq!(
            table.vote_result(name),
            (expected.clone(), Some(table.id(&werewolf)))
        );
    }
    assert!(!table.is_alive(&werewolf));
    assert_eq!(
        table.summary("P2", &werewolf).revealed_role,
        Some(Role::Werewolf)
    );
}

#[test]
fn a_player_can_change_their_vote_until_the_vote_ends() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();

    table.vote("P1", "P2");
    table.vote("P1", "P3");
    table.time_runs_out();

    let (ballots, eliminated) = table.vote_result("P1");
    assert_eq!(ballots, vec![table.ballot("P1", Some("P3"))]);
    assert_eq!(eliminated, Some(table.id("P3")));
}

#[test]
fn a_player_can_vote_for_themselves() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();

    table.vote("P4", "P4");
    table.time_runs_out();

    assert_eq!(table.vote_result("P1").1, Some(table.id("P4")));
    assert!(!table.is_alive("P4"));
}

#[test]
fn abstentions_count_as_votes_cast_but_designate_nobody() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();

    for voter in ["P1", "P2", "P3", "P4"] {
        table.abstain(voter);
    }
    table.vote("P5", "P2");

    let (ballots, eliminated) = table.vote_result("P1");
    assert_eq!(ballots.len(), 5);
    assert_eq!(ballots[0], table.ballot("P1", None));
    assert_eq!(eliminated, Some(table.id("P2")));
}

#[test]
fn a_tied_vote_eliminates_nobody() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();

    table.vote("P1", "P2");
    table.vote("P2", "P1");
    table.time_runs_out();

    assert_eq!(table.vote_result("P3").1, None);
    assert!(table.everyone().iter().all(|p| table.is_alive(p)));
}

#[test]
fn a_vote_nobody_cast_eliminates_nobody() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();

    table.time_runs_out();

    assert_eq!(table.vote_result("P3"), (vec![], None));
}

#[test]
fn night_falls_again_after_the_vote_result() {
    let mut table = Table::start(7, 2);
    table.skip_to_the_vote();
    table.time_runs_out();
    assert_eq!(table.outputs.timer().unwrap().seconds, 10);

    table.time_runs_out();

    let werewolf = table.werewolves()[0].to_string();
    assert_eq!(table.picks_seen_by(&werewolf), Some(vec![]));
}

impl Table {
    fn winner(&self, viewer: &str) -> Camp {
        match self.moment(viewer) {
            Moment::Victory { winner } => *winner,
            other => panic!("expected victory, got {other:?}"),
        }
    }

    fn assert_every_role_revealed(&self) {
        for viewer in self.everyone() {
            for (name, role) in &self.players {
                assert_eq!(
                    self.summary(&viewer, name).revealed_role,
                    Some(*role),
                    "{viewer} sees {name}"
                );
            }
        }
    }
}

#[test]
fn the_village_wins_once_every_werewolf_is_dead() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();
    let werewolf = table.werewolves()[0].to_string();
    for voter in table.everyone() {
        table.vote(&voter, &werewolf);
    }

    table.time_runs_out();

    for name in table.everyone() {
        assert_eq!(table.winner(&name), Camp::Village);
    }
    assert_eq!(table.outputs.timer(), None);
    table.assert_every_role_revealed();
}

impl Table {
    fn living(&self) -> Vec<String> {
        self.everyone()
            .into_iter()
            .filter(|p| self.is_alive(p))
            .collect()
    }
}

#[test]
fn the_werewolves_win_once_they_are_as_many_as_the_others() {
    let mut table = Table::start(5, 1);
    let werewolf = table.werewolves()[0].to_string();
    let villagers: Vec<String> = table.villagers().iter().map(|v| v.to_string()).collect();

    table.pick(&werewolf, &villagers[0]);
    table.skip_to_the_vote();
    for voter in table.living() {
        table.vote(&voter, &villagers[1]);
    }
    table.time_runs_out();
    table.pick(&werewolf, &villagers[2]);
    assert_eq!(table.dawn_deaths(), vec![table.id(&villagers[2])]);

    table.time_runs_out();

    assert_eq!(table.winner("P1"), Camp::Werewolves);
    table.assert_every_role_revealed();
}

impl Table {
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

    fn pick_command(&self, victim: &str) -> Command {
        Command::PickVictim {
            victim: self.id(victim),
        }
    }

    fn vote_command(&self, designated: &str) -> Command {
        Command::Vote {
            designated: Some(self.id(designated)),
        }
    }
}

#[test]
fn only_the_living_werewolves_pick_and_only_during_their_turn() {
    let mut table = Table::start(7, 2);
    let [first, second, villager, other] = table.two_werewolves_and_two_villagers();

    let command = table.pick_command(&other);
    assert_eq!(table.refused(&villager, command), Rejection::NotYourTurn);
    let command = table.vote_command(&other);
    assert_eq!(table.refused(&villager, command), Rejection::NotNow);

    table.pick(&first, &second);
    table.pick(&second, &second);
    let command = table.pick_command(&other);
    assert_eq!(table.refused(&first, command), Rejection::NotNow);

    table.skip_to_the_vote();
    table.time_runs_out();
    table.time_runs_out();
    let command = table.pick_command(&other);
    assert_eq!(table.refused(&second, command), Rejection::Spectating);
}

/// The id of a Player seated in another Lobby, larger than this Game's.
fn stranger() -> PlayerId {
    let mut other = Engine::new(LobbyCode::new("AUTRE"), Seeded(1));
    let names: Vec<String> = (1..=12).map(|n| format!("S{n}")).collect();
    let mut last = None;
    for name in &names {
        last = Some(other.handle(&seat(name), join(name)).unwrap());
    }
    last.unwrap().view_for(&seat("S12")).unwrap().you
}

#[test]
fn nobody_can_choose_an_eliminated_player_or_a_stranger() {
    let mut table = Table::start(7, 2);
    let [first, second, villager, other] = table.two_werewolves_and_two_villagers();
    table.pick(&first, &villager);
    table.pick(&second, &villager);
    table.skip_to_the_vote();

    let command = table.vote_command(&villager);
    assert_eq!(table.refused(&other, command), Rejection::NotInPlay);
    table.time_runs_out();
    table.time_runs_out();
    let command = table.pick_command(&villager);
    assert_eq!(table.refused(&first, command), Rejection::NotInPlay);
    let unknown = Command::PickVictim { victim: stranger() };
    assert_eq!(table.refused(&first, unknown), Rejection::NotInPlay);
}

#[test]
fn the_dead_cannot_vote() {
    let mut table = Table::start(7, 2);
    let [first, second, villager, other] = table.two_werewolves_and_two_villagers();
    table.pick(&first, &villager);
    table.pick(&second, &villager);
    table.skip_to_the_vote();

    let command = table.vote_command(&other);
    assert_eq!(table.refused(&villager, command), Rejection::Spectating);
}

#[test]
fn a_deadline_for_a_moment_that_already_ended_is_ignored() {
    let mut table = Table::start(7, 2);
    let stale = table.outputs.timer().unwrap().moment;
    let [first, second, villager, _] = table.two_werewolves_and_two_villagers();
    table.pick(&first, &villager);
    table.pick(&second, &villager);

    assert_eq!(table.engine.deadline_passed(stale), Err(Rejection::NotNow));
    assert!(matches!(table.moment("P1"), Moment::Dawn { .. }));
}

impl Table {
    /// Plays until the Village wins by voting out the lone Werewolf.
    fn village_wins(&mut self) {
        self.skip_to_the_vote();
        let werewolf = self.werewolves()[0].to_string();
        for voter in self.everyone() {
            self.vote(&voter, &werewolf);
        }
        self.time_runs_out();
    }
}

#[test]
fn play_again_brings_everyone_back_to_the_same_lobby() {
    let mut table = Table::start(5, 1);
    table.village_wins();

    table.act("P1", Command::PlayAgain);

    for name in table.everyone() {
        let view = table.view(&name);
        assert_eq!(view.code.as_str(), "LOUPS");
        assert_eq!(view.moment, None);
        assert_eq!(view.role, None);
        assert_eq!(view.players.len(), 5);
        assert!(view
            .players
            .iter()
            .all(|p| p.alive && p.revealed_role.is_none()));
        assert_eq!(view.start_blocked_by, None);
    }
    assert_eq!(table.outputs.timer(), None);
    table.act("P1", Command::Start);
    assert!(matches!(table.moment("P2"), Moment::WerewolvesTurn { .. }));
}

#[test]
fn only_the_host_can_play_again_and_only_once_the_game_is_over() {
    let mut table = Table::start(5, 1);
    assert_eq!(table.refused("P1", Command::PlayAgain), Rejection::NotNow);

    table.village_wins();

    assert_eq!(table.refused("P2", Command::PlayAgain), Rejection::NotHost);
}

#[test]
fn during_the_vote_each_player_sees_only_their_own_ballot() {
    let mut table = Table::start(5, 1);
    table.skip_to_the_vote();

    table.vote("P1", "P2");
    table.abstain("P3");

    let own_ballot = |table: &Table, name: &str| match table.moment(name) {
        Moment::Vote { your_ballot, .. } => *your_ballot,
        other => panic!("expected the Vote, got {other:?}"),
    };
    assert_eq!(
        own_ballot(&table, "P1"),
        Some(table.ballot("P1", Some("P2")))
    );
    assert_eq!(own_ballot(&table, "P3"), Some(table.ballot("P3", None)));
    assert_eq!(own_ballot(&table, "P2"), None);
}

#[test]
fn a_command_out_of_turn_is_refused_as_such_whoever_it_designates() {
    let mut table = Table::start(7, 2);
    let [first, second, villager, other] = table.two_werewolves_and_two_villagers();
    table.pick(&first, &villager);
    table.pick(&second, &villager);
    table.skip_to_the_vote();
    table.time_runs_out();
    table.time_runs_out();

    let command = table.pick_command(&villager);
    assert_eq!(table.refused(&other, command), Rejection::NotYourTurn);
}
