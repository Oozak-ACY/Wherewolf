//! The Little Girl, scripted only through the engine's public commands and outputs.

mod common;

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Rejection,
    Role, RoleCard, RoleCounts, SeatToken, Settings,
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

/// Two Werewolves, the Little Girl, a Seer, a Witch and two Villagers.
const WITH_LITTLE_GIRL: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 1,
    witch: 1,
    hunter: 0,
    cupid: 0,
    thief: 0,
    little_girl: 1,
    assassin: 0,
    villager: 2,
};

/// A Lobby of `players` Players, P1 (the Host) onwards, not started yet.
fn lobby(seed: u64, players: usize) -> (Engine, Settings) {
    let mut engine = Engine::new(LobbyCode::new("LOUPS"), Seeded(seed));
    let mut outputs = None;
    for n in 1..=players {
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

/// A running Game, on the first Night.
struct Table {
    engine: Engine,
    outputs: Outputs,
    /// Each Player with the Role they were dealt.
    dealt: Vec<(String, Role)>,
}

impl Table {
    fn start() -> Self {
        Self::start_with(WITH_LITTLE_GIRL, false)
    }

    fn start_with(roles: RoleCounts, hidden_roles: bool) -> Self {
        let players = roles.total();
        let (mut engine, mut settings) = lobby(11, players);
        settings.roles = roles;
        settings.hidden_roles = hidden_roles;
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let mut outputs = engine.handle(&seat("P1"), Command::Start).unwrap();
        common::announce(&mut engine, &mut outputs);
        let names: Vec<String> = (1..=players).map(|n| format!("P{n}")).collect();
        let dealt = names
            .iter()
            .map(|name| {
                let role = outputs.view_for(&seat(name)).unwrap().role.as_ref();
                (name.clone(), role.unwrap().role)
            })
            .collect();
        for name in &names {
            outputs = engine.handle(&seat(name), Command::ConfirmRole).unwrap();
        }
        common::announce(&mut engine, &mut outputs);
        Self {
            engine,
            outputs,
            dealt,
        }
    }

    /// The (first) Player dealt `role`.
    fn dealt(&self, role: Role) -> String {
        self.dealt
            .iter()
            .find(|(_, r)| *r == role)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| panic!("nobody was dealt {role:?}"))
    }

    fn little_girl(&self) -> String {
        self.dealt(Role::LittleGirl)
    }

    fn everyone(&self) -> Vec<String> {
        self.dealt.iter().map(|(name, _)| name.clone()).collect()
    }

    fn view(&self, name: &str) -> &PlayerView {
        self.outputs
            .view_for(&seat(name))
            .unwrap_or_else(|| panic!("no view for {name}"))
    }

    fn moment(&self) -> &Moment {
        self.view("P1")
            .moment
            .as_ref()
            .expect("the Game is running")
    }

    fn card(&self, name: &str) -> &RoleCard {
        self.view(name).role.as_ref().expect("a dealt Role")
    }

    fn id(&self, name: &str) -> PlayerId {
        self.view(name).you
    }

    fn alive(&self, name: &str) -> bool {
        let id = self.id(name);
        self.view("P1")
            .players
            .iter()
            .any(|p| p.id == id && p.alive)
    }

    /// The Role `name` reveals to `witness`, now that they are eliminated.
    fn revealed_to(&self, witness: &str, name: &str) -> Option<Role> {
        let id = self.id(name);
        self.view(witness)
            .players
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.revealed_role)
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

    /// Lets the time run out until `reached` holds for the current Moment.
    fn until(&mut self, reached: impl Fn(&Moment) -> bool) {
        for _ in 0..50 {
            if reached(self.moment()) {
                return;
            }
            self.time_runs_out();
        }
        panic!("never reached: stuck at {:?}", self.moment());
    }

    /// Lets the time run out until the next Night's first Turn: the Werewolves
    /// pick nobody and the village votes nobody out.
    fn until_the_next_night(&mut self) {
        self.until(|m| matches!(m, Moment::VoteResult { .. }));
        self.time_runs_out();
    }

    /// Every living Werewolf picks `victim`.
    fn werewolves_kill(&mut self, victim: &str) {
        self.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
        let victim = self.id(victim);
        let werewolves: Vec<String> = self
            .everyone()
            .into_iter()
            .filter(|n| self.alive(n) && self.card(n).role == Role::Werewolf)
            .collect();
        for werewolf in werewolves {
            self.act(&werewolf, Command::PickVictim { victim });
        }
    }

    /// The Turns of the rest of this Night, in order, letting each one's
    /// time run out, up to the dawn.
    fn rest_of_the_night(&mut self) -> Vec<&'static str> {
        let mut turns = Vec::new();
        while !matches!(self.moment(), Moment::Dawn { .. }) {
            turns.push(turn_name(self.moment()));
            self.time_runs_out();
        }
        turns
    }
}

fn turn_name(moment: &Moment) -> &'static str {
    match moment {
        Moment::CupidsTurn => "Cupid",
        Moment::LoversTurn { .. } => "Lovers",
        Moment::ThiefsTurn => "Thief",
        Moment::SeersTurn { .. } => "Seer",
        Moment::WerewolvesTurn { .. } => "Werewolves",
        Moment::WitchsTurn { .. } => "Witch",
        other => panic!("not a Night Turn: {other:?}"),
    }
}

#[test]
fn the_host_can_put_one_little_girl_in_the_game_but_not_two() {
    let (mut engine, mut settings) = lobby(11, 7);

    settings.roles = WITH_LITTLE_GIRL;
    let outputs = engine
        .handle(
            &seat("P1"),
            Command::UpdateSettings {
                settings: settings.clone(),
            },
        )
        .unwrap();
    let view = outputs.view_for(&seat("P1")).unwrap();
    assert_eq!(view.settings.roles.little_girl, 1);
    assert_eq!(view.start_blocked_by, None);

    settings.roles.little_girl = 2;
    settings.roles.villager = 1;
    assert_eq!(
        engine.handle(&seat("P1"), Command::UpdateSettings { settings }),
        Err(Rejection::InvalidSettings)
    );
}

#[test]
fn the_narrator_suggests_no_little_girl() {
    for players in 5..=15 {
        assert_eq!(
            RoleCounts::suggested(players).little_girl,
            0,
            "{players} Players"
        );
    }
}

#[test]
fn the_little_girl_is_dealt_a_village_card() {
    let table = Table::start();
    let little_girl = table.little_girl();

    assert_eq!(Role::LittleGirl.camp(), Camp::Village);
    let card = table.card(&little_girl);
    assert_eq!(card.role, Role::LittleGirl);
    assert_eq!(card.camp, Camp::Village);
    assert!(card.fellow_werewolves.is_empty());
    assert_eq!(card.potions, None);
}

#[test]
fn she_has_no_turn_and_the_night_order_is_unchanged() {
    let mut with_her = Table::start();
    let mut without_her = Table::start_with(
        RoleCounts {
            little_girl: 0,
            assassin: 0,
            villager: 3,
            ..WITH_LITTLE_GIRL
        },
        false,
    );

    let night = with_her.rest_of_the_night();

    assert_eq!(night, ["Seer", "Werewolves", "Witch"]);
    assert_eq!(night, without_her.rest_of_the_night());
}

#[test]
fn the_werewolves_turn_keeps_its_timer_and_ends_once_they_agree() {
    let mut table = Table::start();
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    assert_eq!(table.outputs.timer().unwrap().seconds, 60);

    let villager = table.dealt(Role::Villager);
    table.werewolves_kill(&villager);

    assert!(matches!(table.moment(), Moment::WitchsTurn { .. }));
}

#[test]
fn the_werewolves_can_make_her_their_victim_and_her_role_is_revealed() {
    let mut table = Table::start();
    let little_girl = table.little_girl();

    table.werewolves_kill(&little_girl);
    table.until(|m| matches!(m, Moment::Dawn { .. }));

    let Moment::Dawn { deaths, .. } = table.moment() else {
        unreachable!()
    };
    assert_eq!(deaths, &vec![table.id(&little_girl)]);
    assert!(!table.alive(&little_girl));
    for name in table.everyone() {
        assert_eq!(
            table.revealed_to(&name, &little_girl),
            Some(Role::LittleGirl),
            "{name}"
        );
    }
}

#[test]
fn with_hidden_roles_on_her_role_stays_hidden_and_her_death_adds_no_feigned_turn() {
    let mut table = Table::start_with(WITH_LITTLE_GIRL, true);
    let little_girl = table.little_girl();

    table.werewolves_kill(&little_girl);
    table.until(|m| matches!(m, Moment::Dawn { .. }));
    for name in table.everyone() {
        if name != little_girl {
            assert_eq!(table.revealed_to(&name, &little_girl), None, "{name}");
        }
    }

    table.until_the_next_night();
    assert_eq!(table.rest_of_the_night(), ["Seer", "Werewolves", "Witch"]);
}

#[test]
fn a_thief_who_robs_her_takes_the_little_girl_card() {
    let mut table = Table::start_with(
        RoleCounts {
            thief: 1,
            villager: 1,
            ..WITH_LITTLE_GIRL
        },
        false,
    );
    table.until_the_next_night();
    let thief = table.dealt(Role::Thief);
    let little_girl = table.little_girl();
    let player = table.id(&little_girl);
    table.act(&thief, Command::Rob { player });
    table.until(|m| matches!(m, Moment::Discussion));

    assert_eq!(table.card(&thief).role, Role::LittleGirl);
    assert_eq!(table.card(&thief).camp, Camp::Village);
    assert_eq!(table.card(&little_girl).role, Role::Villager);
}
