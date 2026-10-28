//! The Lovers' bond in death: when one Lover is eliminated, the other dies of
//! grief, as a death trigger. And their Camp: a Werewolf and a non-Werewolf
//! form the Lovers Camp, which wins as the last two alive. Scripted only
//! through the engine's public commands and outputs.

mod common;

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Moment, Outputs, PlayerId, PlayerView, Randomness, Role,
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

/// Two Werewolves, a Witch, a Hunter, Cupid and four Villagers.
const ROLES: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 1,
    hunter: 1,
    cupid: 1,
    thief: 0,
    little_girl: 0,
    assassin: 0,
    villager: 4,
};

/// A running Game with one Player per Role card, P1 (the Host) onwards, on
/// the first Night, the Lovers named.
struct Table {
    engine: Engine,
    outputs: Outputs,
    players: Vec<(String, Role)>,
}

impl Table {
    /// The Lovers are the Players `lovers` picks among the dealt Roles.
    fn start(hidden_roles: bool, lovers: impl Fn(&Table) -> [String; 2]) -> Self {
        Self::start_with(ROLES, hidden_roles, lovers)
    }

    /// The same, with `roles` dealt instead.
    fn start_with(
        roles: RoleCounts,
        hidden_roles: bool,
        lovers: impl Fn(&Table) -> [String; 2],
    ) -> Self {
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
        settings.hidden_roles = hidden_roles;
        engine
            .handle(&seat("P1"), Command::UpdateSettings { settings })
            .unwrap();
        let mut started = engine.handle(&seat("P1"), Command::Start).unwrap();
        common::announce(&mut engine, &mut started);
        // Everyone confirms their Role: the first Night falls.
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
        let mut table = Self {
            engine,
            outputs,
            players,
        };
        let [a, b] = lovers(&table);
        let cupid = table.with_role(Role::Cupid)[0].clone();
        let lovers = vec![table.id(&a), table.id(&b)];
        table.act(&cupid, Command::ChooseLovers { lovers });
        // The Lovers' Turn runs out: the Werewolves wake.
        table.time_runs_out();
        assert!(matches!(table.moment("P1"), Moment::WerewolvesTurn { .. }));
        table
    }

    fn with_role(&self, role: Role) -> Vec<String> {
        self.players
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn villager(&self, n: usize) -> String {
        self.with_role(Role::Villager)[n].clone()
    }

    fn werewolf(&self, n: usize) -> String {
        self.with_role(Role::Werewolf)[n].clone()
    }

    fn cupid(&self) -> String {
        self.with_role(Role::Cupid)[0].clone()
    }

    fn hunter(&self) -> String {
        self.with_role(Role::Hunter)[0].clone()
    }

    fn witch(&self) -> String {
        self.with_role(Role::Witch)[0].clone()
    }

    fn everyone(&self) -> Vec<String> {
        self.players.iter().map(|(name, _)| name.clone()).collect()
    }

    fn living(&self) -> Vec<String> {
        self.everyone()
            .into_iter()
            .filter(|p| self.alive(p))
            .collect()
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

    /// Asserts every Player sees the same Moment, and returns it.
    fn moment_for_all(&self) -> Moment {
        let everyone = self.everyone();
        let moment = self.moment(&everyone[0]).clone();
        for name in &everyone {
            assert_eq!(self.moment(name), &moment, "{name} sees another Moment");
        }
        moment
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

    /// From the Werewolves' Turn to the dawn: the Werewolves kill `victim`
    /// (or nobody), and the Witch poisons `poisoned` (or nobody).
    fn night(&mut self, victim: Option<&str>, poisoned: Option<&str>) {
        assert!(matches!(self.moment("P1"), Moment::WerewolvesTurn { .. }));
        match victim {
            Some(victim) => {
                let victim = self.id(victim);
                let werewolves: Vec<String> = self
                    .with_role(Role::Werewolf)
                    .into_iter()
                    .filter(|w| self.alive(w))
                    .collect();
                for werewolf in werewolves {
                    self.act(&werewolf, Command::PickVictim { victim });
                }
            }
            None => self.time_runs_out(),
        }
        while matches!(self.moment("P1"), Moment::WerewolvesTurn { .. }) {
            self.time_runs_out();
        }
        if let Some(poisoned) = poisoned {
            let player = self.id(poisoned);
            let witch = self.witch();
            self.act(&witch, Command::Poison { player });
        }
        while !matches!(self.moment("P1"), Moment::Dawn { .. }) {
            self.time_runs_out();
        }
    }

    /// From the dawn to the Vote: on the first Day, every living Player
    /// elects `mayor`.
    fn until_the_vote(&mut self, mayor: &str) {
        loop {
            match self.moment("P1") {
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

    /// Every living Player votes against `designated`.
    fn village_eliminates(&mut self, designated: &str) {
        let designated = Some(self.id(designated));
        for voter in self.living() {
            self.act(&voter, Command::Vote { designated });
        }
        assert!(matches!(
            self.moment("P1"),
            Moment::VoteResult {
                eliminated: Some(_),
                ..
            }
        ));
    }

    /// The Vote eliminates `designated`, and its result is announced.
    fn vote_out(&mut self, designated: &str) {
        self.village_eliminates(designated);
        self.time_runs_out();
    }

    /// The Camp on `name`'s Role card.
    fn camp(&self, name: &str) -> Camp {
        self.view(name).role.as_ref().unwrap().camp
    }

    /// The Lovers as `viewer` sees them, sorted.
    fn lovers_seen_by(&self, viewer: &str) -> Vec<PlayerId> {
        let mut lovers = self.view(viewer).lovers.clone();
        lovers.sort();
        lovers
    }

    fn grief(&self, lover: &str) -> Moment {
        Moment::Grief {
            lover: self.id(lover),
        }
    }
}

/// Two Villagers.
fn two_villagers(table: &Table) -> [String; 2] {
    [table.villager(0), table.villager(1)]
}

#[test]
fn a_lover_killed_by_the_werewolves_dies_at_dawn_and_the_partner_dies_of_grief_right_after() {
    let mut table = Table::start(false, two_villagers);
    let (killed, grieving) = (table.villager(0), table.villager(1));

    table.night(Some(&killed), None);
    assert_eq!(
        table.moment_for_all(),
        Moment::Dawn {
            deaths: vec![table.id(&killed)],
            robbery: false,
        }
    );
    assert!(table.alive(&grieving));

    table.time_runs_out();
    assert_eq!(table.moment_for_all(), table.grief(&grieving));
    assert!(!table.alive(&grieving));
    for viewer in table.everyone() {
        assert_eq!(
            table.revealed_to(&viewer, &grieving),
            Some(Role::Villager),
            "{viewer}"
        );
    }

    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
}

#[test]
fn a_lover_eliminated_by_the_vote_takes_the_partner_with_them_before_nightfall() {
    let mut table = Table::start(false, two_villagers);
    let (eliminated, grieving) = (table.villager(0), table.villager(1));
    table.night(None, None);
    table.until_the_vote(&table.villager(2));

    table.village_eliminates(&eliminated);
    assert!(table.alive(&grieving));
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), table.grief(&grieving));
    assert!(!table.alive(&grieving));
    table.time_runs_out();
    assert!(matches!(
        table.moment(&table.villager(2)),
        Moment::WerewolvesTurn { .. }
    ));
}

#[test]
fn a_lover_poisoned_by_the_witch_dies_at_dawn_and_the_partner_dies_of_grief() {
    let mut table = Table::start(false, two_villagers);
    let (poisoned, grieving) = (table.villager(0), table.villager(1));

    table.night(None, Some(&poisoned));
    assert_eq!(
        table.moment_for_all(),
        Moment::Dawn {
            deaths: vec![table.id(&poisoned)],
            robbery: false,
        }
    );
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), table.grief(&grieving));
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
}

#[test]
fn a_lover_shot_by_the_hunter_takes_the_partner_with_them() {
    let mut table = Table::start(false, two_villagers);
    let (hunter, shot, grieving) = (table.hunter(), table.villager(0), table.villager(1));
    table.night(Some(&hunter), None);
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::HuntersShot { .. }));

    let player = table.id(&shot);
    table.act(&hunter, Command::Shoot { player });
    assert!(table.alive(&grieving));
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), table.grief(&grieving));
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
}

#[test]
fn a_hunter_who_dies_of_grief_shoots_next() {
    let mut table = Table::start(false, |t| [t.villager(0), t.hunter()]);
    let (killed, hunter, shot) = (table.villager(0), table.hunter(), table.villager(1));

    table.night(Some(&killed), None);
    table.time_runs_out();
    assert_eq!(table.moment_for_all(), table.grief(&hunter));
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    let player = table.id(&shot);
    table.act(&hunter, Command::Shoot { player });
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
}

#[test]
fn a_lover_who_is_the_hunter_shoots_after_the_partner_dies_of_grief() {
    let mut table = Table::start(false, |t| [t.villager(0), t.hunter()]);
    let (hunter, grieving) = (table.hunter(), table.villager(0));

    table.night(Some(&hunter), None);
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), table.grief(&grieving));
    assert!(!table.view(&hunter).spectating, "waiting for his shot");
    table.time_runs_out();
    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
}

#[test]
fn a_mayor_who_dies_of_grief_names_a_successor_next() {
    let mut table = Table::start(false, two_villagers);
    let (eliminated, mayor, successor) = (table.villager(0), table.villager(1), table.villager(2));
    table.night(None, None);
    table.until_the_vote(&mayor);
    assert_eq!(table.view("P1").mayor, Some(table.id(&mayor)));

    table.village_eliminates(&eliminated);
    table.time_runs_out();
    assert_eq!(table.moment_for_all(), table.grief(&mayor));
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Succession {
            mayor: table.id(&mayor)
        }
    );
    let player = table.id(&successor);
    table.act(&mayor, Command::NameSuccessor { player });
    assert_eq!(table.view("P1").mayor, Some(table.id(&successor)));
    table.time_runs_out();
    assert!(matches!(
        table.moment(&successor),
        Moment::WerewolvesTurn { .. }
    ));
}

#[test]
fn both_lovers_dying_the_same_night_die_at_dawn_and_grief_kills_nobody_twice() {
    let mut table = Table::start(false, two_villagers);
    let (killed, poisoned) = (table.villager(0), table.villager(1));

    table.night(Some(&killed), Some(&poisoned));
    let Moment::Dawn { deaths, .. } = table.moment_for_all() else {
        panic!("the village wakes up");
    };
    let mut both = vec![table.id(&killed), table.id(&poisoned)];
    both.sort();
    assert_eq!(deaths, both);

    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
}

#[test]
fn a_hunter_lover_dying_the_same_night_as_the_partner_still_shoots_and_grief_kills_nobody() {
    let mut table = Table::start(false, |t| [t.villager(0), t.hunter()]);
    let (killed, hunter, shot) = (table.villager(0), table.hunter(), table.villager(1));

    table.night(Some(&killed), Some(&hunter));
    table.time_runs_out();
    assert_eq!(
        table.moment_for_all(),
        Moment::HuntersShot {
            hunter: table.id(&hunter)
        }
    );
    let player = table.id(&shot);
    table.act(&hunter, Command::Shoot { player });
    table.time_runs_out();

    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
}

#[test]
fn with_hidden_roles_on_the_grieving_players_role_is_not_revealed_to_the_living() {
    let mut table = Table::start(true, two_villagers);
    let (killed, grieving) = (table.villager(0), table.villager(1));

    table.night(Some(&killed), None);
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), table.grief(&grieving));
    for viewer in table.living() {
        assert_eq!(table.revealed_to(&viewer, &grieving), None, "{viewer}");
    }
}

#[test]
fn the_grief_is_announced_for_as_long_as_any_result() {
    let mut table = Table::start(false, two_villagers);
    table.night(Some(&table.villager(0)), None);
    table.time_runs_out();

    assert_eq!(
        table.outputs.timer().unwrap().seconds,
        wherewolf_engine::ANNOUNCEMENT_SECONDS
    );
    for viewer in table.living() {
        assert!(table.view(&viewer).receives_anyone(), "{viewer}");
    }
}

/// Two Werewolves, Cupid and two Villagers.
const SMALL: RoleCounts = RoleCounts {
    werewolf: 2,
    seer: 0,
    witch: 0,
    hunter: 0,
    cupid: 1,
    thief: 0,
    little_girl: 0,
    assassin: 0,
    villager: 2,
};

/// The first Werewolf and the first Villager: a mixed couple.
fn werewolf_and_villager(table: &Table) -> [String; 2] {
    [table.werewolf(0), table.villager(0)]
}

#[test]
fn a_werewolf_and_a_villager_lover_are_in_the_lovers_camp() {
    let table = Table::start(false, werewolf_and_villager);
    let (wolf, villager) = (table.werewolf(0), table.villager(0));

    assert_eq!(table.camp(&wolf), Camp::Lovers);
    assert_eq!(table.camp(&villager), Camp::Lovers);
    assert_eq!(table.camp(&table.werewolf(1)), Camp::Werewolves);
    assert_eq!(table.camp(&table.villager(1)), Camp::Village);
}

#[test]
fn two_village_lovers_stay_village_and_two_werewolf_lovers_stay_werewolves() {
    let village = Table::start(false, two_villagers);
    assert_eq!(village.camp(&village.villager(0)), Camp::Village);
    assert_eq!(village.camp(&village.villager(1)), Camp::Village);

    let pack = Table::start(false, |t| [t.werewolf(0), t.werewolf(1)]);
    assert_eq!(pack.camp(&pack.werewolf(0)), Camp::Werewolves);
    assert_eq!(pack.camp(&pack.werewolf(1)), Camp::Werewolves);
}

#[test]
fn a_werewolf_lover_keeps_their_role_and_still_wakes_with_the_pack() {
    let table = Table::start(false, werewolf_and_villager);
    let wolf = table.werewolf(0);

    assert_eq!(
        table.view(&wolf).role.as_ref().unwrap().role,
        Role::Werewolf
    );
    assert!(table.view(&wolf).receives_anyone());
}

#[test]
fn a_mixed_couple_as_the_last_two_alive_wins_and_everyone_sees_who_they_are() {
    let mut table = Table::start_with(SMALL, false, werewolf_and_villager);
    let (wolf, cupid) = (table.werewolf(1), table.cupid());
    table.night(Some(&table.villager(1)), None);
    table.until_the_vote(&cupid);
    table.vote_out(&wolf);

    table.night(Some(&cupid), None);
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Victory {
            winner: Some(Camp::Lovers)
        }
    );
    let couple = table.lovers_seen_by(&table.werewolf(0));
    assert_eq!(couple.len(), 2);
    for viewer in table.everyone() {
        assert_eq!(table.lovers_seen_by(&viewer), couple, "{viewer}");
    }
}

#[test]
fn one_ordinary_werewolf_against_a_mixed_couple_has_not_won_until_the_couple_votes_it_out() {
    let mut table = Table::start_with(SMALL, false, werewolf_and_villager);
    let (wolf, cupid) = (table.werewolf(1), table.cupid());
    table.night(Some(&table.villager(1)), None);
    table.until_the_vote(&table.villager(0));
    table.vote_out(&cupid);
    // One Werewolf outside the couple, against both Lovers: the Night falls.
    assert!(matches!(
        table.moment(&table.villager(0)),
        Moment::WerewolvesTurn { .. }
    ));

    table.night(None, None);
    table.until_the_vote(&table.villager(0));
    table.vote_out(&wolf);

    assert_eq!(
        table.moment_for_all(),
        Moment::Victory {
            winner: Some(Camp::Lovers)
        }
    );
}

#[test]
fn killing_the_werewolf_lover_with_no_other_werewolf_left_wins_for_the_village() {
    let mut table = Table::start_with(SMALL, false, werewolf_and_villager);
    let (lover, wolf, cupid) = (table.werewolf(0), table.werewolf(1), table.cupid());
    table.night(Some(&table.villager(1)), None);
    table.until_the_vote(&cupid);
    table.vote_out(&wolf);
    table.night(None, None);
    table.until_the_vote(&cupid);

    table.vote_out(&lover);
    assert_eq!(table.moment_for_all(), table.grief(&table.villager(0)));
    table.time_runs_out();

    assert_eq!(
        table.moment_for_all(),
        Moment::Victory {
            winner: Some(Camp::Village)
        }
    );
}

#[test]
fn the_werewolves_outside_the_couple_win_once_they_are_as_many_as_everyone_else() {
    let roles = RoleCounts {
        werewolf: 3,
        ..SMALL
    };
    let mut table = Table::start_with(roles, false, werewolf_and_villager);
    let cupid = table.cupid();
    table.night(Some(&table.villager(1)), None);
    // Two Werewolves outside the couple, against the two Lovers and Cupid.
    table.time_runs_out();
    assert!(matches!(table.moment_for_all(), Moment::Election { .. }));
    table.until_the_vote(&cupid);

    table.vote_out(&cupid);

    assert_eq!(
        table.moment_for_all(),
        Moment::Victory {
            winner: Some(Camp::Werewolves)
        }
    );
}

#[test]
fn a_chain_of_triggers_that_kills_the_last_living_players_is_a_draw() {
    let roles = RoleCounts {
        hunter: 1,
        villager: 1,
        ..SMALL
    };
    let mut table = Table::start_with(roles, false, |t| [t.werewolf(0), t.hunter()]);
    let (lover, wolf, hunter, villager) = (
        table.werewolf(0),
        table.werewolf(1),
        table.hunter(),
        table.villager(0),
    );
    table.night(Some(&table.cupid()), None);
    table.until_the_vote(&villager);
    table.vote_out(&wolf);
    table.night(None, None);
    table.until_the_vote(&villager);

    // The Hunter's partner dies of grief, then he shoots the last one, the Mayor.
    table.vote_out(&hunter);
    assert_eq!(table.moment_for_all(), table.grief(&lover));
    table.time_runs_out();
    let player = table.id(&villager);
    table.act(&hunter, Command::Shoot { player });
    table.time_runs_out();

    assert_eq!(table.moment_for_all(), Moment::Victory { winner: None });
    assert!(table.living().is_empty());
}
