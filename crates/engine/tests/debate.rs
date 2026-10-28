//! The Day's order (the Election first on the first Day) and ending the
//! debate early, which only the Mayor may do, scripted only through the
//! engine's public commands and outputs.

mod common;

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

/// Two Werewolves and five Villagers: no Night Turn but the Werewolves'.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 0,
    hunter: 0,
    cupid: 0,
    thief: 0,
    little_girl: 0,
    assassin: 0,
    villager: 5,
};

/// The Host.
const HOST: &str = "P1";

/// A running Game with one Player per Role card, P1 (the Host) onwards, on
/// the first Night's Werewolves' Turn.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        Self::start_with(ROLES)
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
            .view_for(&seat(HOST))
            .unwrap()
            .settings
            .clone();
        settings.roles = roles;
        engine
            .handle(&seat(HOST), Command::UpdateSettings { settings })
            .unwrap();
        let mut started = engine.handle(&seat(HOST), Command::Start).unwrap();
        common::announce(&mut engine, &mut started);
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

    fn everyone(&self) -> Vec<String> {
        self.players.iter().map(|(name, _)| name.clone()).collect()
    }

    /// The Villagers other than the Host, in seat order.
    fn villager(&self, n: usize) -> String {
        self.players
            .iter()
            .filter(|(name, role)| *role == Role::Villager && name != HOST)
            .nth(n)
            .unwrap()
            .0
            .clone()
    }

    fn view(&self, name: &str) -> &PlayerView {
        self.outputs.view_for(&seat(name)).unwrap()
    }

    fn moment(&self) -> &Moment {
        self.view(HOST)
            .moment
            .as_ref()
            .expect("the Game is running")
    }

    fn id(&self, name: &str) -> PlayerId {
        self.view(name).you
    }

    fn alive(&self, name: &str) -> bool {
        let id = self.id(name);
        self.view(HOST)
            .players
            .iter()
            .any(|p| p.id == id && p.alive)
    }

    fn living(&self) -> Vec<String> {
        self.everyone()
            .into_iter()
            .filter(|p| self.alive(p))
            .collect()
    }

    /// Who sees the button that ends the debate.
    fn may_end_debate(&self) -> Vec<String> {
        self.everyone()
            .into_iter()
            .filter(|p| self.view(p).may_end_debate)
            .collect()
    }

    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
        self.outputs = self.engine.deadline_passed(timer.moment).unwrap();
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

    /// During the Werewolves' Turn, every living Werewolf picks `victim`;
    /// the Night ends and the village wakes up.
    fn werewolves_kill(&mut self, victim: &str) {
        let victim = self.id(victim);
        let werewolves: Vec<String> = self
            .players
            .iter()
            .filter(|(name, role)| *role == Role::Werewolf && self.alive(name))
            .map(|(name, _)| name.clone())
            .collect();
        for werewolf in werewolves {
            self.act(&werewolf, Command::PickVictim { victim });
        }
    }

    /// During the Election, every living Player votes for `mayor`.
    fn everyone_elects(&mut self, mayor: &str) {
        let candidate = self.id(mayor);
        for voter in self.living() {
            self.act(&voter, Command::Elect { candidate });
        }
    }

    /// From the first Night: the Werewolves kill `victim`, and the first
    /// Day's Election begins.
    fn first_election(&mut self, victim: &str) {
        self.werewolves_kill(victim);
        // The dawn.
        self.time_runs_out();
        assert!(matches!(self.moment(), Moment::Election { voted, .. } if voted.is_empty()));
    }

    /// From the first Night: the Werewolves kill a Villager, `mayor` is
    /// elected, and the first Day's debate begins.
    fn first_debate(&mut self, mayor: &str) {
        self.first_election(&self.villager(0));
        self.everyone_elects(mayor);
        // The Election's result.
        self.time_runs_out();
        assert_eq!(self.moment(), &Moment::Discussion);
    }

    /// From the first Night: `mayor` is elected on the first Day, nobody is
    /// eliminated by the Vote, and the second Day's debate begins.
    fn second_debate(&mut self, mayor: &str) {
        self.first_debate(mayor);
        // The debate, the Vote nobody votes in, its result.
        self.time_runs_out();
        self.time_runs_out();
        self.time_runs_out();
        self.werewolves_kill(&self.villager(1));
        // The dawn.
        self.time_runs_out();
        assert_eq!(self.moment(), &Moment::Discussion);
    }
}

#[test]
fn the_first_day_elects_the_mayor_then_debates_then_votes() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    table.first_election(&table.villager(0));
    assert_eq!(table.view(HOST).mayor, None);

    table.everyone_elects(&mayor);
    assert!(matches!(table.moment(), Moment::ElectionResult { .. }));
    table.time_runs_out();
    assert_eq!(table.moment(), &Moment::Discussion);
    assert_eq!(table.view(HOST).mayor, Some(table.id(&mayor)));
    table.time_runs_out();

    assert!(matches!(table.moment(), Moment::Vote { voted, .. } if voted.is_empty()));
}

#[test]
fn a_later_day_debates_then_votes_without_an_election() {
    let mut table = Table::start();
    table.second_debate(&table.villager(2));

    table.time_runs_out();

    assert!(matches!(table.moment(), Moment::Vote { .. }));
}

#[test]
fn the_mayor_ends_the_first_days_debate_and_the_vote_begins() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    table.first_debate(&mayor);
    let debate = table.outputs.timer().unwrap().moment;

    table.act(&mayor, Command::EndDebate);

    assert!(matches!(table.moment(), Moment::Vote { voted, .. } if voted.is_empty()));
    // The debate's timer no longer ends anything.
    assert_eq!(table.engine.deadline_passed(debate), Err(Rejection::NotNow));
}

#[test]
fn the_mayor_ends_a_later_debate_and_the_vote_begins() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    table.second_debate(&mayor);

    table.act(&mayor, Command::EndDebate);

    assert!(matches!(table.moment(), Moment::Vote { voted, .. } if voted.is_empty()));
}

#[test]
fn the_host_who_is_not_mayor_cannot_end_the_debate() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    table.first_debate(&mayor);

    assert_eq!(
        table.refused(HOST, Command::EndDebate),
        Rejection::NotYourTurn
    );
    assert_eq!(
        table.refused(&table.villager(3), Command::EndDebate),
        Rejection::NotYourTurn
    );
}

#[test]
fn the_debate_cannot_be_ended_outside_the_debate() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    // The Night.
    assert_eq!(table.refused(HOST, Command::EndDebate), Rejection::NotNow);
    table.first_election(&table.villager(0));
    // The Election.
    assert_eq!(table.refused(HOST, Command::EndDebate), Rejection::NotNow);
    assert_eq!(table.refused(&mayor, Command::EndDebate), Rejection::NotNow);
}

#[test]
fn a_spectator_cannot_end_the_debate() {
    let mut table = Table::start();
    let victim = table.villager(0);
    table.first_debate(&table.villager(2));

    assert_eq!(
        table.refused(&victim, Command::EndDebate),
        Rejection::Spectating
    );
}

#[test]
fn a_host_killed_on_the_first_night_cannot_end_the_debate() {
    let mut table = Table::start();
    assert_eq!(table.players[0].1, Role::Villager, "the Host can be killed");
    table.werewolves_kill(HOST);
    table.time_runs_out();
    let mayor = table.villager(2);
    table.everyone_elects(&mayor);
    table.time_runs_out();
    assert_eq!(table.moment(), &Moment::Discussion);
    assert!(!table.alive(HOST));

    assert_eq!(table.may_end_debate(), vec![mayor]);
    assert_eq!(
        table.refused(HOST, Command::EndDebate),
        Rejection::Spectating
    );
}

#[test]
fn only_the_mayor_sees_the_button_that_ends_the_debate() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    assert!(table.may_end_debate().is_empty(), "nobody, at Night");
    table.first_election(&table.villager(0));
    assert!(
        table.may_end_debate().is_empty(),
        "nobody, during the Election"
    );
    table.everyone_elects(&mayor);
    table.time_runs_out();
    assert_eq!(table.may_end_debate(), vec![mayor.clone()]);
    table.act(&mayor, Command::EndDebate);
    assert!(table.may_end_debate().is_empty(), "nobody, during the Vote");
}

/// Two Werewolves and three Villagers: one Villager killed on the first
/// Night leaves the Werewolves as many as the others.
#[test]
fn a_game_decided_at_the_first_dawn_never_opens_the_election() {
    let roles = RoleCounts {
        villager: 3,
        ..ROLES
    };
    let mut table = Table::start_with(roles);

    table.werewolves_kill(&table.villager(0));
    table.time_runs_out();

    assert!(matches!(table.moment(), Moment::Victory { .. }));
    assert_eq!(table.view(HOST).mayor, None);
}
