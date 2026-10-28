//! The Assassin, scripted only through the engine's public commands and outputs.

mod common;

use wherewolf_engine::{
    Camp, Command, Engine, Inspection, LobbyCode, Moment, Outputs, PlayerId, PlayerView,
    Randomness, Rejection, Role, RoleCard, RoleCounts, SeatToken, Settings,
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

/// Two Werewolves, the Assassin, a Seer, a Witch and two Villagers.
const WITH_ASSASSIN: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 1,
    witch: 1,
    hunter: 0,
    cupid: 0,
    thief: 0,
    little_girl: 0,
    assassin: 1,
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
        Self::start_with(WITH_ASSASSIN, false)
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

    fn assassin(&self) -> String {
        self.dealt(Role::Assassin)
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

    fn living(&self) -> Vec<String> {
        self.everyone()
            .into_iter()
            .filter(|n| self.alive(n))
            .collect()
    }

    /// Everyone dealt `role`, in seat order.
    fn with_role(&self, role: Role) -> Vec<String> {
        self.dealt
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Cupid binds `first` and `second`, and the Lovers' Turn runs out.
    fn bind(&mut self, first: &str, second: &str) {
        self.until(|m| matches!(m, Moment::CupidsTurn));
        let lovers = vec![self.id(first), self.id(second)];
        let cupid = self.dealt(Role::Cupid);
        self.act(&cupid, Command::ChooseLovers { lovers });
        self.time_runs_out();
    }

    /// The rest of this Night up to the dawn, the Werewolves killing
    /// `victim`, or nobody.
    fn night(&mut self, victim: Option<&str>) {
        if let Some(victim) = victim {
            self.werewolves_kill(victim);
        }
        self.until(|m| matches!(m, Moment::Dawn { .. }));
    }

    /// From the dawn to the Vote: on the first Day, every living Player
    /// elects `mayor`.
    fn until_the_vote(&mut self, mayor: &str) {
        loop {
            match self.moment() {
                Moment::Vote { .. } => return,
                Moment::Election { .. } => {
                    let candidate = self.id(mayor);
                    for voter in self.living() {
                        self.act(&voter, Command::Elect { candidate });
                    }
                }
                _ => self.time_runs_out(),
            }
        }
    }

    /// Every living Player votes against `designated`, and the result is
    /// announced.
    fn vote_out(&mut self, designated: &str) {
        let designated = Some(self.id(designated));
        for voter in self.living() {
            self.act(&voter, Command::Vote { designated });
        }
        self.time_runs_out();
    }

    /// The last two alive vote against each other, the Mayor eliminates
    /// `designated`, and the result is announced.
    fn tie_broken_against(&mut self, mayor: &str, designated: &str) {
        let living = self.living();
        assert_eq!(living.len(), 2, "the last two alive");
        for (voter, other) in [(&living[0], &living[1]), (&living[1], &living[0])] {
            let designated = Some(self.id(other));
            self.act(voter, Command::Vote { designated });
        }
        assert!(matches!(self.moment(), Moment::TieBreak { .. }));
        let player = self.id(designated);
        self.act(mayor, Command::BreakTie { player });
        self.time_runs_out();
    }

    /// Lets the rest of the Night run out, and who dies at its dawn. `None`
    /// if the Game ended first.
    fn dawn_deaths(&mut self) -> Option<Vec<PlayerId>> {
        self.until(|m| matches!(m, Moment::Dawn { .. } | Moment::Victory { .. }));
        match self.moment() {
            Moment::Dawn { deaths, .. } => Some(deaths.clone()),
            _ => None,
        }
    }

    /// The winner, once the Game is over: `Some(None)` for a draw.
    fn victory(&self) -> Option<Option<Camp>> {
        match self.moment() {
            Moment::Victory { winner } => Some(*winner),
            _ => None,
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
        Moment::AssassinsTurn => "Assassin",
        Moment::WitchsTurn { .. } => "Witch",
        other => panic!("not a Night Turn: {other:?}"),
    }
}

#[test]
fn the_host_can_put_one_assassin_in_the_game_but_not_two() {
    let (mut engine, mut settings) = lobby(11, 7);

    settings.roles = WITH_ASSASSIN;
    let outputs = engine
        .handle(
            &seat("P1"),
            Command::UpdateSettings {
                settings: settings.clone(),
            },
        )
        .unwrap();
    let view = outputs.view_for(&seat("P1")).unwrap();
    assert_eq!(view.settings.roles.assassin, 1);
    assert_eq!(view.start_blocked_by, None);

    settings.roles.assassin = 2;
    settings.roles.villager = 1;
    assert_eq!(
        engine.handle(&seat("P1"), Command::UpdateSettings { settings }),
        Err(Rejection::InvalidSettings)
    );
}

#[test]
fn the_narrator_suggests_no_assassin() {
    for players in 5..=12 {
        assert_eq!(
            RoleCounts::suggested(players).assassin,
            0,
            "{players} Players"
        );
    }
}

#[test]
fn the_assassin_is_dealt_a_card_of_their_own_camp() {
    let table = Table::start();
    let assassin = table.assassin();

    assert_eq!(Role::Assassin.camp(), Camp::Assassin);
    let card = table.card(&assassin);
    assert_eq!(card.role, Role::Assassin);
    assert_eq!(card.camp, Camp::Assassin);
    assert!(card.fellow_werewolves.is_empty());
    assert_eq!(card.potions, None);
}

#[test]
fn the_assassin_wakes_every_night_right_after_the_werewolves() {
    let mut with_assassin = Table::start();
    let mut without_assassin = Table::start_with(
        RoleCounts {
            assassin: 0,
            villager: 3,
            ..WITH_ASSASSIN
        },
        false,
    );

    assert_eq!(
        with_assassin.rest_of_the_night(),
        ["Seer", "Werewolves", "Assassin", "Witch"]
    );
    assert_eq!(
        without_assassin.rest_of_the_night(),
        ["Seer", "Werewolves", "Witch"]
    );

    with_assassin.until(|m| matches!(m, Moment::SeersTurn { .. }));
    assert_eq!(
        with_assassin.rest_of_the_night(),
        ["Seer", "Werewolves", "Assassin", "Witch"],
        "the second Night"
    );
}

#[test]
fn the_assassins_turn_lasts_their_timer_which_defaults_to_thirty_seconds() {
    let mut table = Table::start();
    assert_eq!(table.view("P1").settings.timers.assassin, 30);

    table.until(|m| matches!(m, Moment::AssassinsTurn));

    assert_eq!(table.outputs.timer().map(|t| t.seconds), Some(30));
}

#[test]
fn the_assassins_target_dies_at_dawn_and_their_turn_ends_on_confirming() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();
    let villager = table.dealt(Role::Villager);
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    let player = table.id(&villager);
    table.act(&assassin, Command::Kill { player });

    assert_eq!(table.dawn_deaths(), Some(vec![player]));
    assert!(!table.alive(&villager));
}

#[test]
fn the_assassin_can_kill_a_werewolf() {
    let mut table = Table::start_with(DUEL, false);
    let (wolf, assassin) = (table.dealt(Role::Werewolf), table.assassin());
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    let player = table.id(&wolf);
    table.act(&assassin, Command::Kill { player });

    assert_eq!(table.dawn_deaths(), Some(vec![player]));
}

#[test]
fn only_the_assassin_kills_and_never_themself_nor_the_dead() {
    let mut table = Table::start_with(DUEL, false);
    let (wolf, assassin) = (table.dealt(Role::Werewolf), table.assassin());
    let villagers = table.with_role(Role::Villager);
    table.werewolves_kill(&villagers[0]);
    table.until(|m| matches!(m, Moment::Dawn { .. }));
    let (dead, living, itself) = (
        table.id(&villagers[0]),
        table.id(&villagers[1]),
        table.id(&assassin),
    );
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    let mut kill = |name: &str, player| table.engine.handle(&seat(name), Command::Kill { player });
    assert_eq!(kill(&assassin, itself), Err(Rejection::Yourself));
    assert_eq!(kill(&assassin, dead), Err(Rejection::NotInPlay));
    assert_eq!(kill(&wolf, living), Err(Rejection::NotYourTurn));
    assert_eq!(kill(&villagers[0], living), Err(Rejection::Spectating));
}

#[test]
fn the_assassin_kills_only_in_their_turn() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));

    let player = table.id(&table.dealt(Role::Villager));
    assert_eq!(
        table
            .engine
            .handle(&seat(&assassin), Command::Kill { player }),
        Err(Rejection::NotNow)
    );
}

#[test]
fn passing_kills_nobody() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    table.act(&assassin, Command::Pass);

    assert_eq!(table.dawn_deaths(), Some(vec![]));
}

#[test]
fn the_assassins_time_running_out_kills_nobody() {
    let mut table = Table::start_with(DUEL, false);
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), Some(vec![]));
}

#[test]
fn the_werewolves_can_make_the_assassin_their_victim() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();

    table.werewolves_kill(&assassin);
    table.time_runs_out();

    assert_eq!(table.dawn_deaths(), Some(vec![table.id(&assassin)]));
}

#[test]
fn a_player_chosen_by_both_dies_once() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();
    let villager = table.dealt(Role::Villager);
    table.werewolves_kill(&villager);

    let player = table.id(&villager);
    table.act(&assassin, Command::Kill { player });

    assert_eq!(table.dawn_deaths(), Some(vec![player]));
}

#[test]
fn the_victim_and_the_target_both_die_named_in_seat_order() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();
    let villagers = table.with_role(Role::Villager);
    table.werewolves_kill(&villagers[1]);

    let player = table.id(&villagers[0]);
    table.act(&assassin, Command::Kill { player });

    let mut both = vec![table.id(&villagers[0]), table.id(&villagers[1])];
    both.sort();
    assert_eq!(table.dawn_deaths(), Some(both));
}

#[test]
fn the_witch_sees_the_target_among_the_dying_and_may_save_them() {
    let mut table = Table::start();
    let (assassin, witch) = (table.assassin(), table.dealt(Role::Witch));
    let villagers = table.with_role(Role::Villager);
    table.werewolves_kill(&villagers[1]);
    let target = table.id(&villagers[0]);
    table.act(&assassin, Command::Kill { player: target });

    let mut dying = vec![target, table.id(&villagers[1])];
    dying.sort();
    let Some(Moment::WitchsTurn { witch: Some(sight) }) = table.view(&witch).moment.clone() else {
        panic!("the Witch's Turn, as she sees it");
    };
    assert_eq!(sight.dying, dying);

    table.act(&witch, Command::Heal { player: target });
    table.act(&witch, Command::Pass);

    assert_eq!(table.dawn_deaths(), Some(vec![table.id(&villagers[1])]));
}

#[test]
fn one_potion_saves_a_player_chosen_by_both() {
    let mut table = Table::start();
    let (assassin, witch) = (table.assassin(), table.dealt(Role::Witch));
    let villager = table.dealt(Role::Villager);
    table.werewolves_kill(&villager);
    let player = table.id(&villager);
    table.act(&assassin, Command::Kill { player });

    table.act(&witch, Command::Heal { player });
    table.act(&witch, Command::Pass);

    assert_eq!(table.dawn_deaths(), Some(vec![]));
}

#[test]
fn spectators_see_the_assassins_turn_and_their_choice_but_the_living_do_not() {
    let mut table = Table::start_with(DUEL, false);
    let assassin = table.assassin();
    let villagers = table.with_role(Role::Villager);
    table.night(Some(&villagers[0]));
    table.until(|m| matches!(m, Moment::AssassinsTurn));
    assert_eq!(
        table.view(&villagers[0]).moment,
        Some(Moment::AssassinsTurn)
    );

    let player = table.id(&villagers[1]);
    table.act(&assassin, Command::Kill { player });

    assert_eq!(table.view(&villagers[0]).assassin_target, Some(player));
    for name in [&assassin, &villagers[1], &villagers[2]] {
        assert_eq!(table.view(name).assassin_target, None, "{name}");
    }
}

#[test]
fn the_assassin_acts_alone_seeing_and_heard_by_nobody() {
    let mut table = Table::start();
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    for name in table.everyone() {
        assert!(table.view(&name).receives.is_empty(), "{name}");
    }
}

#[test]
fn with_hidden_roles_off_a_dead_assassins_turn_is_skipped() {
    let mut table = Table::start();
    let assassin = table.assassin();
    table.night(Some(&assassin));
    table.until(|m| matches!(m, Moment::SeersTurn { .. }));

    assert_eq!(table.rest_of_the_night(), ["Seer", "Werewolves", "Witch"]);
}

#[test]
fn with_hidden_roles_on_a_dead_assassins_turn_is_feigned() {
    let mut table = Table::start_with(WITH_ASSASSIN, true);
    let assassin = table.assassin();
    table.night(Some(&assassin));
    table.until(|m| matches!(m, Moment::AssassinsTurn));

    let seconds = table.outputs.timer().unwrap().seconds;
    assert!((10..=30).contains(&seconds), "{seconds}");
    let player = table.id(&table.dealt(Role::Witch));
    assert_eq!(
        table
            .engine
            .handle(&seat(&assassin), Command::Kill { player }),
        Err(Rejection::Spectating)
    );
}

#[test]
fn the_seer_inspecting_the_assassin_sees_assassin() {
    let mut table = Table::start();
    let seer = table.dealt(Role::Seer);
    let assassin = table.assassin();
    let player = table.id(&assassin);

    table.act(&seer, Command::Inspect { player });

    let Some(Moment::SeersTurn { inspection, .. }) = table.view(&seer).moment.as_ref() else {
        panic!("the Seer's Turn is still open");
    };
    assert_eq!(
        *inspection,
        Some(Inspection {
            player,
            role: Role::Assassin,
        })
    );
}

#[test]
fn an_eliminated_assassin_is_revealed_to_everyone() {
    let mut table = Table::start();
    let assassin = table.assassin();

    table.werewolves_kill(&assassin);
    table.until(|m| matches!(m, Moment::Dawn { .. }));

    assert!(!table.alive(&assassin));
    for name in table.everyone() {
        assert_eq!(
            table.revealed_to(&name, &assassin),
            Some(Role::Assassin),
            "{name}"
        );
    }
}

#[test]
fn with_hidden_roles_on_an_eliminated_assassin_stays_hidden() {
    let mut table = Table::start_with(WITH_ASSASSIN, true);
    let assassin = table.assassin();

    table.werewolves_kill(&assassin);
    table.until(|m| matches!(m, Moment::Dawn { .. }));

    assert!(!table.alive(&assassin));
    for name in table.everyone() {
        if name != assassin {
            assert_eq!(table.revealed_to(&name, &assassin), None, "{name}");
        }
    }
}

/// One Werewolf, the Assassin and three Villagers.
const DUEL: RoleCounts = RoleCounts {
    werewolf: 1,
    seer: 0,
    witch: 0,
    hunter: 0,
    cupid: 0,
    thief: 0,
    little_girl: 0,
    assassin: 1,
    villager: 3,
};

/// Cupid, one Werewolf, the Assassin and two Villagers.
const WITH_CUPID: RoleCounts = RoleCounts {
    cupid: 1,
    villager: 2,
    ..DUEL
};

#[test]
fn with_no_werewolf_left_the_game_goes_on_until_the_assassin_alone_survives() {
    let mut table = Table::start_with(DUEL, false);
    let (wolf, assassin) = (table.dealt(Role::Werewolf), table.assassin());
    let villagers = table.with_role(Role::Villager);
    table.night(Some(&villagers[0]));
    table.until_the_vote(&assassin);

    table.vote_out(&wolf);
    assert_eq!(table.victory(), None, "the Assassin still lives");

    table.night(None);
    table.until_the_vote(&assassin);
    table.vote_out(&villagers[1]);
    assert_eq!(table.victory(), None);

    // The Day opens with the Assassin and one other Player: the Mayor
    // breaks the tie.
    table.night(None);
    table.until_the_vote(&assassin);
    table.tie_broken_against(&assassin, &villagers[2]);

    assert_eq!(table.victory(), Some(Some(Camp::Assassin)));
    assert_eq!(table.living(), [assassin]);
}

#[test]
fn the_werewolves_at_parity_have_not_won_while_the_assassin_lives() {
    let mut table = Table::start_with(DUEL, false);
    let (wolf, assassin) = (table.dealt(Role::Werewolf), table.assassin());
    let villagers = table.with_role(Role::Villager);
    table.night(Some(&villagers[0]));
    table.until_the_vote(&wolf);
    table.vote_out(&villagers[1]);

    // One Werewolf and the Assassin left at dawn: the Day goes on.
    table.night(Some(&villagers[2]));
    table.time_runs_out();
    assert_eq!(table.victory(), None);

    // The Werewolf, as Mayor, breaks the tie.
    table.until_the_vote(&wolf);
    table.tie_broken_against(&wolf, &assassin);

    assert_eq!(table.victory(), Some(Some(Camp::Werewolves)));
}

#[test]
fn the_assassin_dead_the_werewolves_win_at_parity() {
    let roles = RoleCounts {
        werewolf: 2,
        villager: 2,
        ..DUEL
    };
    let mut table = Table::start_with(roles, false);
    let assassin = table.assassin();

    table.night(Some(&assassin));
    table.time_runs_out();

    assert_eq!(table.victory(), Some(Some(Camp::Werewolves)));
}

#[test]
fn the_village_wins_once_every_werewolf_and_the_assassin_are_dead() {
    let mut table = Table::start_with(DUEL, false);
    let (wolf, assassin) = (table.dealt(Role::Werewolf), table.assassin());
    let villager = table.dealt(Role::Villager);
    table.night(Some(&assassin));
    table.until_the_vote(&villager);

    table.vote_out(&wolf);

    assert_eq!(table.victory(), Some(Some(Camp::Village)));
}

#[test]
fn an_assassin_bound_to_a_villager_wins_with_them_as_the_last_two_alive() {
    let mut table = Table::start_with(WITH_CUPID, false);
    let (wolf, assassin, cupid) = (
        table.dealt(Role::Werewolf),
        table.assassin(),
        table.dealt(Role::Cupid),
    );
    let villagers = table.with_role(Role::Villager);
    table.bind(&assassin, &villagers[0]);
    assert_eq!(table.card(&assassin).camp, Camp::Lovers);
    assert_eq!(table.card(&villagers[0]).camp, Camp::Lovers);

    table.night(Some(&cupid));
    table.until_the_vote(&assassin);
    table.vote_out(&wolf);
    assert_eq!(table.victory(), None, "the Assassin still lives");

    table.night(None);
    table.until_the_vote(&assassin);
    table.vote_out(&villagers[1]);

    assert_eq!(table.victory(), Some(Some(Camp::Lovers)));
}

#[test]
fn a_werewolf_bound_to_the_assassin_loses_to_the_village_once_both_are_dead() {
    let roles = RoleCounts {
        werewolf: 2,
        ..WITH_CUPID
    };
    let mut table = Table::start_with(roles, false);
    let (assassin, cupid) = (table.assassin(), table.dealt(Role::Cupid));
    let wolves = table.with_role(Role::Werewolf);
    let villager = table.dealt(Role::Villager);
    table.bind(&wolves[0], &assassin);
    assert_eq!(table.card(&wolves[0]).camp, Camp::Lovers);
    assert_eq!(table.card(&assassin).camp, Camp::Lovers);

    table.night(Some(&cupid));
    table.until_the_vote(&villager);
    table.vote_out(&wolves[1]);
    assert_eq!(table.victory(), None);

    table.night(None);
    table.until_the_vote(&villager);
    table.vote_out(&wolves[0]);
    assert!(matches!(table.moment(), Moment::Grief { .. }));
    table.time_runs_out();

    assert_eq!(table.victory(), Some(Some(Camp::Village)));
}

#[test]
fn a_werewolf_bound_to_the_assassin_wins_with_them_as_the_last_two_alive() {
    let mut table = Table::start_with(WITH_CUPID, false);
    let (wolf, assassin, cupid) = (
        table.dealt(Role::Werewolf),
        table.assassin(),
        table.dealt(Role::Cupid),
    );
    let villagers = table.with_role(Role::Villager);
    table.bind(&wolf, &assassin);

    table.night(Some(&cupid));
    table.until_the_vote(&villagers[0]);
    table.vote_out(&villagers[0]);
    assert_eq!(table.victory(), None);

    table.night(Some(&villagers[1]));
    table.time_runs_out();

    assert_eq!(table.victory(), Some(Some(Camp::Lovers)));
}
