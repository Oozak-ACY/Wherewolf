//! The Grace period: in the Lobby, a Player who stays Disconnected that long
//! loses their seat as if they had Left. The engine reads no clock: the
//! server reports how long a Disconnection has lasted. Scripted only through
//! the engine's public commands and outputs.

use wherewolf_engine::{
    Command, Engine, GracePeriod, LobbyCode, Moment, Outputs, PlayerView, Randomness, Rejection,
    Role, SeatToken, Settings,
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

fn player(n: usize) -> String {
    format!("P{n}")
}

/// A Lobby of P1 (the Host) to P`count`.
struct Lobby {
    engine: Engine,
    outputs: Outputs,
}

impl Lobby {
    fn of(count: usize) -> Self {
        let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(7));
        let mut outputs = None;
        for n in 1..=count {
            let name = player(n);
            outputs = Some(engine.handle(&seat(&name), join(&name)).unwrap());
        }
        Self {
            engine,
            outputs: outputs.expect("at least one Player"),
        }
    }

    fn try_act(&mut self, name: &str, command: Command) -> Result<(), Rejection> {
        self.outputs = self.engine.handle(&seat(name), command)?;
        Ok(())
    }

    fn act(&mut self, name: &str, command: Command) {
        self.try_act(name, command)
            .unwrap_or_else(|reason| panic!("{name} was refused: {reason:?}"));
    }

    fn view(&self, name: &str) -> &PlayerView {
        self.outputs
            .view_for(&seat(name))
            .unwrap_or_else(|| panic!("no view for {name}"))
    }

    fn names(&self, viewer: &str) -> Vec<&str> {
        self.view(viewer)
            .players
            .iter()
            .map(|p| p.name.as_str())
            .collect()
    }

    fn host(&self, viewer: &str) -> &str {
        let view = self.view(viewer);
        let host = view.players.iter().find(|p| p.id == view.host).unwrap();
        &host.name
    }

    fn settings(&self) -> Settings {
        self.view("P1").settings.clone()
    }

    fn set_grace_period(&mut self, seconds: u32) -> Result<(), Rejection> {
        let mut settings = self.settings();
        settings.grace_period = seconds;
        self.try_act("P1", Command::UpdateSettings { settings })
    }

    /// The Grace period running for `name`'s Disconnection.
    fn grace_period_of(&self, name: &str) -> GracePeriod {
        let (_, any) = self.outputs.views().next().expect("someone is seated");
        let id = any.players.iter().find(|p| p.name == name).unwrap().id;
        *self
            .outputs
            .grace_periods()
            .iter()
            .find(|g| g.player == id)
            .unwrap_or_else(|| panic!("no Grace period running for {name}"))
    }

    /// The server reports that the Disconnection behind `grace` has lasted
    /// `elapsed` seconds.
    fn report(&mut self, grace: GracePeriod, elapsed: u32) -> Result<(), Rejection> {
        self.outputs = self
            .engine
            .grace_period_passed(grace.disconnection, elapsed)?;
        Ok(())
    }

    /// The server's timer for the current Moment fires.
    fn time_runs_out(&mut self) {
        let timer = self.outputs.timer().expect("the current Moment is timed");
        self.outputs = self.engine.deadline_passed(timer.moment).unwrap();
    }

    fn moment(&self) -> &Moment {
        self.view("P1")
            .moment
            .as_ref()
            .expect("the Game is running")
    }
}

#[test]
fn the_grace_period_is_60_seconds_by_default() {
    let lobby = Lobby::of(1);

    assert_eq!(lobby.settings().grace_period, 60);
}

#[test]
fn the_grace_period_is_bounded_like_the_timers() {
    let mut lobby = Lobby::of(1);

    assert_eq!(lobby.set_grace_period(9), Err(Rejection::InvalidSettings));
    assert_eq!(
        lobby.set_grace_period(1801),
        Err(Rejection::InvalidSettings)
    );
    assert_eq!(lobby.settings().grace_period, 60);

    lobby.set_grace_period(10).unwrap();
    assert_eq!(lobby.settings().grace_period, 10);
    lobby.set_grace_period(1800).unwrap();
    assert_eq!(lobby.settings().grace_period, 1800);
}

#[test]
fn a_disconnected_players_grace_period_runs_for_the_setting() {
    let mut lobby = Lobby::of(3);

    lobby.act("P2", Command::Disconnect);

    assert_eq!(lobby.outputs.grace_periods().len(), 1);
    assert_eq!(lobby.grace_period_of("P2").seconds, 60);
}

#[test]
fn a_player_disconnected_for_the_grace_period_loses_their_seat() {
    let mut lobby = Lobby::of(3);
    lobby.act("P2", Command::Disconnect);
    let grace = lobby.grace_period_of("P2");

    lobby.report(grace, 60).unwrap();

    for name in ["P1", "P3"] {
        assert_eq!(lobby.names(name), ["P1", "P3"], "{name}");
    }
    assert!(lobby.outputs.view_for(&seat("P2")).is_none());
    assert!(lobby.outputs.grace_periods().is_empty());
}

#[test]
fn a_report_before_the_deadline_has_no_effect() {
    let mut lobby = Lobby::of(3);
    lobby.act("P2", Command::Disconnect);
    let grace = lobby.grace_period_of("P2");

    assert_eq!(lobby.report(grace, 59), Err(Rejection::NotNow));
    assert_eq!(lobby.names("P1"), ["P1", "P2", "P3"]);
}

#[test]
fn reconnecting_before_the_deadline_keeps_the_seat_and_the_late_deadline_has_no_effect() {
    let mut lobby = Lobby::of(3);
    lobby.act("P2", Command::Disconnect);
    let grace = lobby.grace_period_of("P2");

    lobby.act("P2", join("P2"));

    assert!(lobby.outputs.grace_periods().is_empty());
    assert_eq!(lobby.report(grace, 60), Err(Rejection::NotNow));
    assert_eq!(lobby.names("P1"), ["P1", "P2", "P3"]);
}

#[test]
fn a_new_disconnection_gets_a_new_grace_period() {
    let mut lobby = Lobby::of(3);
    lobby.act("P2", Command::Disconnect);
    let first = lobby.grace_period_of("P2");
    lobby.act("P2", join("P2"));

    lobby.act("P2", Command::Disconnect);

    let second = lobby.grace_period_of("P2");
    assert_ne!(second.disconnection, first.disconnection);
    assert_eq!(lobby.report(first, 60), Err(Rejection::NotNow));
    lobby.report(second, 60).unwrap();
    assert_eq!(lobby.names("P1"), ["P1", "P3"]);
}

#[test]
fn when_the_hosts_grace_period_runs_out_the_next_player_who_joined_becomes_host() {
    let mut lobby = Lobby::of(3);
    lobby.act("P1", Command::Disconnect);
    let grace = lobby.grace_period_of("P1");

    lobby.report(grace, 60).unwrap();

    assert_eq!(lobby.host("P2"), "P2");
    assert_eq!(lobby.host("P3"), "P2");
}

#[test]
fn lowering_the_grace_period_below_the_time_already_disconnected_frees_the_seat() {
    let mut lobby = Lobby::of(3);
    lobby.act("P2", Command::Disconnect);
    let grace = lobby.grace_period_of("P2");
    assert_eq!(lobby.report(grace, 45), Err(Rejection::NotNow));

    lobby.set_grace_period(30).unwrap();

    let lowered = lobby.grace_period_of("P2");
    assert_eq!(lowered.disconnection, grace.disconnection);
    assert_eq!(lowered.seconds, 30);
    lobby.report(lowered, 45).unwrap();
    assert_eq!(lobby.names("P1"), ["P1", "P3"]);
}

#[test]
fn raising_the_grace_period_extends_the_deadline() {
    let mut lobby = Lobby::of(3);
    lobby.act("P2", Command::Disconnect);

    lobby.set_grace_period(120).unwrap();

    let raised = lobby.grace_period_of("P2");
    assert_eq!(raised.seconds, 120);
    assert_eq!(lobby.report(raised, 60), Err(Rejection::NotNow));
    assert_eq!(lobby.names("P1"), ["P1", "P2", "P3"]);
    lobby.report(raised, 120).unwrap();
    assert_eq!(lobby.names("P1"), ["P1", "P3"]);
}

#[test]
fn a_freed_player_comes_back_as_a_newcomer_at_the_end() {
    let mut lobby = Lobby::of(3);
    lobby.act("P1", Command::Disconnect);
    let grace = lobby.grace_period_of("P1");
    lobby.report(grace, 60).unwrap();

    lobby.act("P1", join("P1"));

    assert_eq!(lobby.names("P1"), ["P2", "P3", "P1"]);
    assert_eq!(lobby.host("P1"), "P2");
}

#[test]
fn a_freed_player_is_refused_when_the_lobby_has_filled_up() {
    let mut lobby = Lobby::of(12);
    lobby.act("P2", Command::Disconnect);
    let grace = lobby.grace_period_of("P2");
    lobby.report(grace, 60).unwrap();
    lobby.act("P13", join("P13"));

    assert_eq!(lobby.try_act("P2", join("P2")), Err(Rejection::LobbyFull));
}

#[test]
fn the_host_cannot_start_while_a_player_is_disconnected() {
    let mut lobby = Lobby::of(5);

    lobby.act("P5", Command::Disconnect);

    assert_eq!(
        lobby.view("P1").start_blocked_by,
        Some(Rejection::PlayersDisconnected)
    );
    assert_eq!(
        lobby.try_act("P1", Command::Start),
        Err(Rejection::PlayersDisconnected)
    );
}

#[test]
fn the_host_can_start_once_the_disconnected_player_is_back() {
    let mut lobby = Lobby::of(5);
    lobby.act("P5", Command::Disconnect);

    lobby.act("P5", join("P5"));

    assert_eq!(lobby.view("P1").start_blocked_by, None);
    lobby.act("P1", Command::Start);
}

#[test]
fn the_host_can_start_once_the_disconnected_players_seat_is_freed() {
    let mut lobby = Lobby::of(6);
    lobby.act("P6", Command::Disconnect);
    let grace = lobby.grace_period_of("P6");

    lobby.report(grace, 60).unwrap();

    assert_eq!(lobby.view("P1").start_blocked_by, None);
    lobby.act("P1", Command::Start);
}

/// From a Lobby of 5, a Game the Village wins on the first Vote, with `disconnected`
/// (not the Werewolf) Disconnected from the start of the Night.
fn play_a_game_with(lobby: &mut Lobby, disconnected: &str) {
    lobby.act("P1", Command::Start);
    for name in (1..=5).map(player) {
        lobby.act(&name, Command::ConfirmRole);
    }
    let werewolf = (1..=5)
        .map(player)
        .find(|n| lobby.view(n).role.as_ref().unwrap().role == Role::Werewolf)
        .unwrap();
    assert_ne!(werewolf, disconnected, "pick another seed");
    lobby.act(disconnected, Command::Disconnect);
    while !matches!(lobby.moment(), Moment::Vote { .. }) {
        lobby.time_runs_out();
    }
    let designated = Some(lobby.view(&werewolf).you);
    for name in (1..=5).map(player) {
        if name != disconnected && name != werewolf {
            lobby.act(&name, Command::Vote { designated });
        }
    }
    while !matches!(lobby.moment(), Moment::Victory { .. }) {
        lobby.time_runs_out();
    }
}

#[test]
fn disconnecting_mid_game_never_frees_a_seat() {
    let mut lobby = Lobby::of(5);

    play_a_game_with(&mut lobby, "P3");

    assert!(lobby.outputs.grace_periods().is_empty());
    assert_eq!(lobby.names("P1"), ["P1", "P2", "P3", "P4", "P5"]);
}

#[test]
fn a_player_still_disconnected_after_play_again_gets_a_full_grace_period_from_the_return() {
    let mut lobby = Lobby::of(5);
    play_a_game_with(&mut lobby, "P3");

    lobby.act("P1", Command::PlayAgain);

    let grace = lobby.grace_period_of("P3");
    assert_eq!(grace.seconds, 60);
    lobby.report(grace, 60).unwrap();
    assert_eq!(lobby.names("P1"), ["P1", "P2", "P4", "P5"]);
}
