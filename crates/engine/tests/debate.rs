//! Ending the Day's debate early: the Host on the first Day, the Mayor once
//! elected, scripted only through the engine's public commands and outputs.

use wherewolf_engine::{
    Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Rejection,
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

/// Two Werewolves and five Villagers: no Night Turn but the Werewolves'.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 0,
    hunter: 0,
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
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(11));
        let names: Vec<String> = (1..=ROLES.total()).map(|n| format!("P{n}")).collect();
        let mut outputs = None;
        for name in &names {
            outputs = Some(engine.handle(&seat(name), join(name)).unwrap());
        }
        let mut settings = outputs.unwrap().view_for(&seat(HOST)).unwrap().settings.clone();
        settings.roles = ROLES;
        engine
            .handle(&seat(HOST), Command::UpdateSettings { settings })
            .unwrap();
        engine.handle(&seat(HOST), Command::Start).unwrap();
        let mut outputs = None;
        for name in &names {
            outputs = Some(engine.handle(&seat(name), Command::ConfirmRole).unwrap());
        }
        let outputs = outputs.unwrap();
        let players = names
            .into_iter()
            .map(|name| {
                let role = outputs.view_for(&seat(&name)).unwrap().role.as_ref().unwrap().role;
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
        self.view(HOST).moment.as_ref().expect("the Game is running")
    }

    fn id(&self, name: &str) -> PlayerId {
        self.view(name).you
    }

    fn alive(&self, name: &str) -> bool {
        let id = self.id(name);
        self.view(HOST).players.iter().any(|p| p.id == id && p.alive)
    }

    fn living(&self) -> Vec<String> {
        self.everyone().into_iter().filter(|p| self.alive(p)).collect()
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

    /// From the first Night: the Werewolves kill `victim`, and the first
    /// Day's debate begins.
    fn first_debate(&mut self, victim: &str) {
        self.werewolves_kill(victim);
        // The dawn.
        self.time_runs_out();
        assert_eq!(self.moment(), &Moment::Discussion);
    }

    /// From the first Night: `mayor` is elected on the first Day, nobody is
    /// eliminated by the Vote, and the second Day's debate begins.
    fn second_debate(&mut self, mayor: &str) {
        self.first_debate(&self.villager(0));
        // The debate runs out: the Election.
        self.time_runs_out();
        let candidate = self.id(mayor);
        for voter in self.living() {
            self.act(&voter, Command::Elect { candidate });
        }
        // The Election's result, the Vote nobody votes in, its result.
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
fn the_host_ends_the_first_days_debate_and_the_election_begins() {
    let mut table = Table::start();
    table.first_debate(&table.villager(0));
    let debate = table.outputs.timer().unwrap().moment;

    table.act(HOST, Command::EndDebate);

    assert!(matches!(table.moment(), Moment::Election { voted, .. } if voted.is_empty()));
    assert_eq!(table.view(HOST).mayor, None);
    // The debate's timer no longer ends anything.
    assert_eq!(table.engine.deadline_passed(debate), Err(Rejection::NotNow));
}

#[test]
fn once_elected_the_mayor_ends_the_debate_and_the_vote_begins() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    table.second_debate(&mayor);

    table.act(&mayor, Command::EndDebate);

    assert!(matches!(table.moment(), Moment::Vote { voted, .. } if voted.is_empty()));
}

#[test]
fn before_the_election_only_the_host_ends_the_debate() {
    let mut table = Table::start();
    table.first_debate(&table.villager(0));

    assert_eq!(
        table.refused(&table.villager(1), Command::EndDebate),
        Rejection::NotYourTurn
    );
}

#[test]
fn once_a_mayor_is_elected_the_host_no_longer_ends_the_debate() {
    let mut table = Table::start();
    let mayor = table.villager(2);
    table.second_debate(&mayor);

    assert_eq!(table.refused(HOST, Command::EndDebate), Rejection::NotYourTurn);
    assert_eq!(
        table.refused(&table.villager(3), Command::EndDebate),
        Rejection::NotYourTurn
    );
}

#[test]
fn the_debate_cannot_be_ended_outside_the_debate() {
    let mut table = Table::start();
    // The Night.
    assert_eq!(table.refused(HOST, Command::EndDebate), Rejection::NotNow);
    table.first_debate(&table.villager(0));
    table.act(HOST, Command::EndDebate);
    // The Election.
    assert_eq!(table.refused(HOST, Command::EndDebate), Rejection::NotNow);
    assert_eq!(
        table.refused(&table.villager(1), Command::EndDebate),
        Rejection::NotNow
    );
}

#[test]
fn a_spectator_cannot_end_the_debate() {
    let mut table = Table::start();
    let victim = table.villager(0);
    table.first_debate(&victim);

    assert_eq!(table.refused(&victim, Command::EndDebate), Rejection::Spectating);
}

#[test]
fn only_the_player_who_may_end_the_debate_sees_the_button() {
    let mut table = Table::start();
    assert!(table.may_end_debate().is_empty(), "nobody, at Night");
    table.first_debate(&table.villager(0));
    assert_eq!(table.may_end_debate(), vec![HOST.to_string()]);
    table.act(HOST, Command::EndDebate);
    assert!(table.may_end_debate().is_empty(), "nobody, during the Election");

    let mut table = Table::start();
    let mayor = table.villager(2);
    table.second_debate(&mayor);
    assert_eq!(table.may_end_debate(), vec![mayor]);
}
