//! The Thief's Turn and the Theft, scripted only through the engine's public commands and outputs.

mod common;

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Potions, Randomness,
    Rejection, Role, RoleCard, RoleCounts, SeatToken, Settings, Theft,
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

/// Two Werewolves, the Thief, a Seer, a Witch, a Hunter and a Villager.
const WITH_THIEF: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 1,
    witch: 1,
    hunter: 1,
    cupid: 0,
    thief: 1,
    villager: 1,
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
        Self::start_with(WITH_THIEF, 11, false)
    }

    fn start_with(roles: RoleCounts, seed: u64, hidden_roles: bool) -> Self {
        let players = roles.total();
        let (mut engine, mut settings) = lobby(seed, players);
        settings.roles = roles;
        settings.hidden_roles = hidden_roles;
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let mut outputs = engine.handle(&seat("P1"), Command::Start).unwrap();
        common::announce(&mut engine, &mut outputs);
        let names: Vec<String> = (1..=players).map(|n| format!("P{n}")).collect();
        for name in &names {
            outputs = engine.handle(&seat(name), Command::ConfirmRole).unwrap();
        }
        common::announce(&mut engine, &mut outputs);
        let dealt = names
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
            dealt,
        }
    }

    /// Every Player dealt `role`.
    fn dealt_all(&self, role: Role) -> Vec<String> {
        self.dealt
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// The (first) Player dealt `role`.
    fn dealt(&self, role: Role) -> String {
        self.dealt_all(role)[0].clone()
    }

    fn thief(&self) -> String {
        self.dealt(Role::Thief)
    }

    fn everyone(&self) -> Vec<String> {
        self.dealt.iter().map(|(name, _)| name.clone()).collect()
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

    fn card(&self, name: &str) -> &RoleCard {
        self.view(name).role.as_ref().expect("a dealt Role")
    }

    fn id(&self, name: &str) -> PlayerId {
        self.view(name).you
    }

    fn alive(&self, name: &str) -> bool {
        let id = self.id(name);
        self.view(name)
            .players
            .iter()
            .any(|p| p.id == id && p.alive)
    }

    /// The Role `name` reveals to everyone, now that they are eliminated.
    fn revealed(&self, name: &str) -> Option<Role> {
        let id = self.id(name);
        self.view("P1")
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

    /// Lets the time run out until `reached` holds for the current Moment.
    fn until(&mut self, reached: impl Fn(&Moment) -> bool) {
        for _ in 0..50 {
            if reached(self.moment("P1")) {
                return;
            }
            self.time_runs_out();
        }
        panic!("never reached: stuck at {:?}", self.moment("P1"));
    }

    /// Lets the time run out until the next Night's first Turn: the Werewolves
    /// pick nobody and the village votes nobody out.
    fn until_the_next_night(&mut self) {
        self.until(|m| matches!(m, Moment::VoteResult { .. }));
        self.time_runs_out();
    }

    /// Lets the time run out until the Day's debate: the dawn's deaths and
    /// their Death triggers are resolved, and the Theft has taken place.
    fn until_the_debate(&mut self) {
        self.until(|m| matches!(m, Moment::Discussion));
    }

    /// The Thief robs `name`.
    fn rob(&mut self, name: &str) {
        let thief = self.thief();
        let player = self.id(name);
        self.act(&thief, Command::Rob { player });
    }

    /// Every living Werewolf (by the card they hold now) picks `victim`.
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

    fn dawn(&self) -> (Vec<PlayerId>, bool) {
        match self.moment("P1") {
            Moment::Dawn { deaths, robbery } => (deaths.clone(), *robbery),
            other => panic!("not the dawn: {other:?}"),
        }
    }
}

#[test]
fn the_host_can_put_one_thief_in_the_game_but_not_two() {
    let (mut engine, mut settings) = lobby(11, 7);

    settings.roles = WITH_THIEF;
    let outputs = engine
        .handle(
            &seat("P1"),
            Command::UpdateSettings {
                settings: settings.clone(),
            },
        )
        .unwrap();
    let view = outputs.view_for(&seat("P1")).unwrap();
    assert_eq!(view.settings.roles.thief, 1);
    assert_eq!(view.start_blocked_by, None);

    settings.roles.thief = 2;
    settings.roles.villager = 0;
    assert_eq!(
        engine.handle(&seat("P1"), Command::UpdateSettings { settings }),
        Err(Rejection::InvalidSettings)
    );
}

#[test]
fn the_thief_counts_toward_the_one_role_per_player_rule() {
    let (mut engine, mut settings) = lobby(11, 7);
    settings.roles = RoleCounts {
        villager: 2,
        ..WITH_THIEF
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
fn the_narrator_suggests_no_thief() {
    for players in 5..=12 {
        assert_eq!(RoleCounts::suggested(players).thief, 0, "{players} Players");
    }
}

#[test]
fn the_thief_is_in_the_village_camp_and_has_their_own_timer() {
    assert_eq!(Role::Thief.camp(), Camp::Village);
    let (_, settings) = lobby(11, 7);
    assert_eq!(settings.timers.thief, 30);
}

#[test]
fn the_thief_never_wakes_on_the_first_night() {
    let table = Table::start();

    assert!(
        matches!(table.moment("P1"), Moment::SeersTurn { .. }),
        "{:?}",
        table.moment("P1")
    );
}

#[test]
fn on_the_second_night_the_thief_wakes_first_and_the_seer_follows() {
    let mut table = Table::start();
    table.until_the_next_night();

    for name in table.everyone() {
        assert_eq!(table.moment(&name), &Moment::ThiefsTurn, "{name}");
    }
    assert_eq!(table.outputs.timer().unwrap().seconds, 30);

    let seer = table.dealt(Role::Seer);
    table.rob(&seer);

    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
}

#[test]
fn the_thief_never_wakes_from_the_third_night_on() {
    let mut table = Table::start();
    table.until_the_next_night();
    assert_eq!(table.moment("P1"), &Moment::ThiefsTurn);

    table.until_the_next_night();

    assert!(
        matches!(table.moment("P1"), Moment::SeersTurn { .. }),
        "{:?}",
        table.moment("P1")
    );
}

#[test]
fn the_thief_acts_alone() {
    let mut table = Table::start();
    table.until_the_next_night();

    for name in table.everyone() {
        assert!(!table.view(&name).receives_anyone(), "{name} sees someone");
    }
    assert!(table.view(&table.thief()).sees_countdown);
}

#[test]
fn the_thief_may_rob_a_living_player_but_not_themself() {
    let mut table = Table::start();
    let thief = table.thief();
    // Someone arriving after the start watches as a Spectator.
    let stranger = table.engine.handle(&seat("P8"), join("P8")).unwrap();
    let stranger = stranger.view_for(&seat("P8")).unwrap().you;
    table.until_the_next_night();

    let yourself = Command::Rob {
        player: table.id(&thief),
    };
    assert_eq!(table.refused(&thief, yourself), Rejection::Yourself);
    let spectator = Command::Rob { player: stranger };
    assert_eq!(table.refused(&thief, spectator), Rejection::NotInPlay);
}

#[test]
fn only_the_thief_robs_and_only_on_the_second_night() {
    let mut table = Table::start();
    let thief = table.thief();
    let seer = table.dealt(Role::Seer);
    let rob_seer = Command::Rob {
        player: table.id(&seer),
    };
    assert_eq!(table.refused(&thief, rob_seer.clone()), Rejection::NotNow);
    table.until_the_next_night();

    let villager = table.dealt(Role::Villager);
    assert_eq!(
        table.refused(&villager, rob_seer.clone()),
        Rejection::NotYourTurn
    );

    table.rob(&seer);
    assert_eq!(table.refused(&thief, rob_seer), Rejection::NotNow);
}

#[test]
fn the_robbed_player_still_acts_with_their_role_that_night() {
    let mut table = Table::start();
    table.until_the_next_night();
    let seer = table.dealt(Role::Seer);
    let thief = table.thief();
    table.rob(&seer);

    let player = table.id(&thief);
    table.act(&seer, Command::Inspect { player });

    let Moment::SeersTurn {
        inspection: Some(inspection),
        ..
    } = table.moment(&seer)
    else {
        panic!("the Seer inspected nobody: {:?}", table.moment(&seer));
    };
    assert_eq!(inspection.role, Role::Thief);
    assert_eq!(table.card(&thief).role, Role::Thief);
    assert_eq!(table.card(&seer).role, Role::Seer);
}

#[test]
fn at_dawn_the_thief_takes_the_robbed_role_and_the_robbed_player_holds_a_villager_card() {
    let mut table = Table::start();
    table.until_the_next_night();
    let seer = table.dealt(Role::Seer);
    let thief = table.thief();
    table.rob(&seer);

    table.until(|m| matches!(m, Moment::Dawn { .. }));
    // Nothing changes until the dawn's deaths are resolved.
    assert_eq!(table.card(&thief).role, Role::Thief);

    table.until_the_debate();
    assert_eq!(table.card(&thief).role, Role::Seer);
    assert_eq!(table.card(&thief).camp, Camp::Village);
    assert_eq!(table.card(&seer).role, Role::Villager);
    assert_eq!(table.card(&seer).camp, Camp::Village);
}

#[test]
fn the_thief_takes_the_witchs_role_as_it_stands_her_spent_potion_staying_spent() {
    let mut table = Table::start();
    let witch = table.dealt(Role::Witch);
    let thief = table.thief();
    let villager = table.dealt(Role::Villager);
    // The first Night, she saves the Villager.
    table.werewolves_kill(&villager);
    table.act(&witch, Command::Heal);
    table.act(&witch, Command::Pass);
    table.until_the_next_night();

    table.rob(&witch);
    table.until_the_debate();

    assert_eq!(table.card(&thief).role, Role::Witch);
    assert_eq!(
        table.card(&thief).potions,
        Some(Potions {
            healing: false,
            poison: true,
        })
    );
    assert_eq!(table.card(&witch).potions, None);

    // The third Night, the Thief is the Witch.
    table.werewolves_kill(&villager);
    assert_eq!(table.refused(&thief, Command::Heal), Rejection::PotionUsed);
    assert_eq!(table.refused(&witch, Command::Heal), Rejection::NotYourTurn);
}

#[test]
fn if_the_time_runs_out_the_thief_robs_nobody_and_plays_as_a_villager() {
    let mut table = Table::start();
    table.until_the_next_night();
    let thief = table.thief();

    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
    table.until_the_debate();

    assert_eq!(table.card(&thief).role, Role::Villager);
    let mut held: Vec<Role> = table
        .everyone()
        .iter()
        .filter(|n| **n != thief)
        .map(|n| table.card(n).role)
        .collect();
    let mut dealt: Vec<Role> = table
        .dealt
        .iter()
        .filter(|(n, _)| *n != thief)
        .map(|(_, r)| *r)
        .collect();
    held.sort_by_key(|r| format!("{r:?}"));
    dealt.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(held, dealt, "nobody else's card changed");
}

#[test]
fn a_thief_who_passes_robs_nobody_and_plays_as_a_villager() {
    let mut table = Table::start();
    table.until_the_next_night();
    let thief = table.thief();

    table.act(&thief, Command::Pass);
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
    table.until_the_debate();

    assert_eq!(table.card(&thief).role, Role::Villager);
}

#[test]
fn the_theft_is_cancelled_if_the_robbed_player_died_that_night_and_a_robbed_hunter_shoots() {
    let mut table = Table::start();
    table.until_the_next_night();
    let hunter = table.dealt(Role::Hunter);
    let thief = table.thief();
    table.rob(&hunter);

    table.werewolves_kill(&hunter);
    table.until(|m| matches!(m, Moment::Dawn { .. }));
    table.time_runs_out();

    assert_eq!(
        table.moment("P1"),
        &Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    table.act(&hunter, Command::Pass);
    table.until_the_debate();
    assert_eq!(table.revealed(&hunter), Some(Role::Hunter));
    assert_eq!(table.card(&thief).role, Role::Villager);
    // A Spectator sees the Theft was cancelled.
    assert_eq!(
        table.view(&hunter).theft,
        Some(Theft {
            thief: table.id(&thief),
            robbed: None,
        })
    );
}

#[test]
fn the_theft_is_cancelled_if_the_thief_died_that_night() {
    let mut table = Table::start();
    table.until_the_next_night();
    let seer = table.dealt(Role::Seer);
    let thief = table.thief();
    table.rob(&seer);

    table.werewolves_kill(&thief);
    table.until_the_debate();

    assert_eq!(table.revealed(&thief), Some(Role::Thief));
    assert_eq!(table.card(&seer).role, Role::Seer);
}

#[test]
fn the_mayor_title_stays_with_the_robbed_player() {
    let mut table = Table::start();
    let seer = table.dealt(Role::Seer);
    table.until(|m| matches!(m, Moment::Election { .. }));
    let candidate = table.id(&seer);
    for name in table.everyone() {
        table.act(&name, Command::Elect { candidate });
    }
    assert_eq!(table.view("P1").mayor, Some(candidate));
    table.until_the_next_night();

    table.rob(&seer);
    table.until_the_debate();

    assert_eq!(table.card(&seer).role, Role::Villager);
    assert_eq!(table.view("P1").mayor, Some(candidate));
}

#[test]
fn the_lovers_bond_stays_with_the_robbed_player() {
    let roles = RoleCounts {
        hunter: 0,
        cupid: 1,
        ..WITH_THIEF
    };
    let mut table = Table::start_with(roles, 11, false);
    let seer = table.dealt(Role::Seer);
    let villager = table.dealt(Role::Villager);
    let thief = table.thief();
    let cupid = table.dealt(Role::Cupid);
    let lovers = vec![table.id(&seer), table.id(&villager)];
    table.act(&cupid, Command::ChooseLovers { lovers });
    table.until_the_next_night();

    table.rob(&seer);
    table.until_the_debate();

    assert_eq!(table.card(&seer).role, Role::Villager);
    assert!(table.view(&seer).lovers.contains(&table.id(&seer)));
    assert_eq!(table.view(&thief).lovers, Vec::<PlayerId>::new());
}

#[test]
fn robbing_a_werewolf_moves_the_thief_into_the_pack_and_the_werewolf_into_the_village() {
    let mut table = Table::start();
    table.until_the_next_night();
    let [robbed, other] = [0, 1].map(|i| table.dealt_all(Role::Werewolf)[i].clone());
    let thief = table.thief();
    table.rob(&robbed);
    table.until_the_debate();

    assert_eq!(table.card(&thief).role, Role::Werewolf);
    assert_eq!(table.card(&thief).camp, Camp::Werewolves);
    assert_eq!(table.card(&thief).fellow_werewolves, vec![table.id(&other)]);
    assert_eq!(table.card(&other).fellow_werewolves, vec![table.id(&thief)]);
    assert_eq!(table.card(&robbed).role, Role::Villager);
    assert_eq!(table.card(&robbed).camp, Camp::Village);

    // At their next Turn, the pack is the Thief and the other Werewolf.
    table.until(|m| matches!(m, Moment::WerewolvesTurn { .. }));
    assert_eq!(table.view(&thief).receives, vec![table.id(&other)]);
    let victim = table.id(&table.dealt(Role::Villager));
    assert_eq!(
        table.refused(&robbed, Command::PickVictim { victim }),
        Rejection::NotYourTurn
    );
    table.act(&thief, Command::PickVictim { victim });
}

#[test]
fn victory_that_dawn_is_checked_with_the_roles_after_the_theft() {
    let roles = RoleCounts {
        werewolf: 2,
        seer: 1,
        witch: 0,
        hunter: 0,
        cupid: 0,
        thief: 1,
        villager: 1,
    };
    let mut table = Table::start_with(roles, 11, false);
    table.until_the_next_night();
    let robbed = table.dealt(Role::Werewolf);
    let thief = table.thief();
    table.rob(&robbed);

    // Two Werewolves are left among four: the pack wins, the Thief in it.
    let villager = table.dealt(Role::Villager);
    table.werewolves_kill(&villager);
    table.until(|m| matches!(m, Moment::Victory { .. }));

    assert_eq!(
        table.moment("P1"),
        &Moment::Victory {
            winner: Some(Camp::Werewolves)
        }
    );
    assert_eq!(table.revealed(&thief), Some(Role::Werewolf));
    assert_eq!(table.revealed(&robbed), Some(Role::Villager));
    for name in table.everyone() {
        assert_eq!(
            table.view(&name).theft,
            Some(Theft {
                thief: table.id(&thief),
                robbed: Some(table.id(&robbed)),
            }),
            "{name}"
        );
    }
}

#[test]
fn nobody_living_is_told_who_robbed_whom() {
    let mut table = Table::start();
    table.engine.handle(&seat("P8"), join("P8")).unwrap();
    table.until_the_next_night();
    let seer = table.dealt(Role::Seer);
    let thief = table.thief();
    table.rob(&seer);
    table.until_the_debate();

    for name in table.everyone() {
        assert_eq!(table.view(&name).theft, None, "{name}");
    }
    assert_eq!(
        table.view("P8").theft,
        Some(Theft {
            thief: table.id(&thief),
            robbed: Some(table.id(&seer)),
        })
    );
}

#[test]
fn an_eliminated_players_revealed_role_is_the_card_they_hold() {
    let mut table = Table::start();
    table.until_the_next_night();
    let seer = table.dealt(Role::Seer);
    table.rob(&seer);
    table.until_the_debate();

    table.until(|m| matches!(m, Moment::Vote { .. }));
    let designated = Some(table.id(&seer));
    for name in table.everyone() {
        table.act(&name, Command::Vote { designated });
    }

    assert!(matches!(table.moment("P1"), Moment::VoteResult { .. }));
    assert_eq!(table.revealed(&seer), Some(Role::Villager));
}

#[test]
fn the_dawn_after_the_second_night_tells_of_a_robbery_even_without_a_theft() {
    let mut table = Table::start();
    table.until(|m| matches!(m, Moment::Dawn { .. }));
    assert!(!table.dawn().1, "no robbery after the first Night");
    table.until_the_next_night();

    let thief = table.thief();
    table.act(&thief, Command::Pass);
    table.until(|m| matches!(m, Moment::Dawn { .. }));
    assert!(table.dawn().1);

    table.until_the_next_night();
    table.until(|m| matches!(m, Moment::Dawn { .. }));
    assert!(!table.dawn().1, "no robbery after the third Night");
}

#[test]
fn the_robbery_is_told_even_when_the_thief_died_before_the_second_night() {
    let mut table = Table::start();
    let thief = table.thief();
    table.werewolves_kill(&thief);
    table.until_the_next_night();
    // With Hidden Roles off, the dead Thief's Turn is skipped.
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));

    table.until(|m| matches!(m, Moment::Dawn { .. }));

    assert!(table.dawn().1);
}

#[test]
fn without_a_thief_no_robbery_is_told() {
    let roles = RoleCounts {
        thief: 0,
        villager: 2,
        ..WITH_THIEF
    };
    let mut table = Table::start_with(roles, 11, false);
    table.until_the_next_night();
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));

    table.until(|m| matches!(m, Moment::Dawn { .. }));

    assert!(!table.dawn().1);
}

#[test]
fn with_hidden_roles_on_a_dead_thiefs_turn_on_the_second_night_is_feigned() {
    let mut table = Table::start_with(WITH_THIEF, 11, true);
    let thief = table.thief();
    table.werewolves_kill(&thief);
    table.until_the_next_night();

    assert_eq!(table.moment("P1"), &Moment::ThiefsTurn);
    let seconds = table.outputs.timer().unwrap().seconds;
    assert!((10..=30).contains(&seconds), "{seconds} s");
    let villager = table.dealt(Role::Villager);
    assert!(!table.view(&villager).sees_countdown);
    let player = table.id(&villager);
    assert_eq!(
        table.refused(&thief, Command::Rob { player }),
        Rejection::Spectating
    );

    table.time_runs_out();
    assert!(matches!(table.moment("P1"), Moment::SeersTurn { .. }));
}
