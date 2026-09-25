//! Settings and Game start, driven only through the engine's public commands
//! and outputs.

use std::collections::HashSet;

use wherewolf_engine::{
    Camp, Command, Engine, LobbyCode, Outputs, PlayerId, PlayerView, Randomness, Rejection, Role,
    RoleCounts, SeatToken, Settings, Timers,
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

fn player(n: usize) -> String {
    format!("P{n}")
}

/// A Lobby with `count` Players, P1 (the Host) to P`count`, and the outputs
/// of the last join.
fn lobby_of(count: usize) -> (Engine, Outputs) {
    seeded_lobby_of(count, 42)
}

fn seeded_lobby_of(count: usize, seed: u64) -> (Engine, Outputs) {
    let mut lobby = Engine::new(LobbyCode::new("LOUPS"), Seeded(seed));
    let mut outputs = None;
    for n in 1..=count {
        let name = player(n);
        outputs = Some(
            lobby
                .handle(&seat(&name), Command::Join { name: name.clone() })
                .unwrap(),
        );
    }
    (lobby, outputs.expect("at least one Player"))
}

fn view<'a>(outputs: &'a Outputs, name: &str) -> &'a PlayerView {
    outputs
        .view_for(&seat(name))
        .unwrap_or_else(|| panic!("no view for {name}"))
}

fn roles(werewolf: u8, seer: u8, witch: u8, hunter: u8, villager: u8) -> RoleCounts {
    RoleCounts {
        werewolf,
        seer,
        witch,
        hunter,
        villager,
    }
}

#[test]
fn the_narrator_suggests_a_role_set_for_the_player_count() {
    let table = [
        (5, roles(1, 1, 1, 0, 2)),
        (6, roles(2, 1, 1, 1, 1)),
        (8, roles(2, 1, 1, 1, 3)),
        (9, roles(3, 1, 1, 1, 3)),
        (12, roles(3, 1, 1, 1, 6)),
    ];
    for (count, expected) in table {
        let (_, outputs) = lobby_of(count);
        assert_eq!(
            view(&outputs, "P1").settings.roles,
            expected,
            "{count} Players"
        );
    }
}

fn settings_seen_by(outputs: &Outputs, name: &str) -> Settings {
    view(outputs, name).settings.clone()
}

#[test]
fn the_timers_start_at_their_defaults() {
    let (_, outputs) = lobby_of(5);

    let timers = settings_seen_by(&outputs, "P1").timers;

    assert_eq!(
        timers,
        Timers {
            discussion: 300,
            election: 60,
            vote: 60,
            seer: 30,
            werewolves: 60,
            witch: 30,
            hunter: 30,
            mayor_successor: 30,
            mayor_tie_break: 30,
        }
    );
}

#[test]
fn the_host_edits_the_settings_and_every_player_sees_them() {
    let (mut lobby, outputs) = lobby_of(6);
    let mut edited = settings_seen_by(&outputs, "P1");
    edited.roles = roles(1, 1, 1, 1, 2);
    edited.timers.discussion = 120;
    edited.timers.werewolves = 45;

    let outputs = lobby
        .handle(
            &seat("P1"),
            Command::UpdateSettings {
                settings: edited.clone(),
            },
        )
        .unwrap();

    for n in 1..=6 {
        assert_eq!(settings_seen_by(&outputs, &player(n)), edited);
    }
}

#[test]
fn only_the_host_can_edit_the_settings() {
    let (mut lobby, outputs) = lobby_of(6);
    let before = settings_seen_by(&outputs, "P1");
    let mut edited = before.clone();
    edited.timers.vote = 90;

    let refused = lobby.handle(&seat("P2"), Command::UpdateSettings { settings: edited });

    assert_eq!(refused, Err(Rejection::NotHost));
    let outputs = lobby.handle(&seat("P2"), join("P2")).unwrap();
    assert_eq!(settings_seen_by(&outputs, "P1"), before);
}

#[test]
fn a_role_set_the_host_edited_is_kept_when_players_come_and_go() {
    let (mut lobby, outputs) = lobby_of(6);
    let mut edited = settings_seen_by(&outputs, "P1");
    edited.roles = roles(1, 1, 1, 1, 2);
    lobby
        .handle(&seat("P1"), Command::UpdateSettings { settings: edited })
        .unwrap();

    let outputs = lobby.handle(&seat("P7"), join("P7")).unwrap();

    assert_eq!(settings_seen_by(&outputs, "P1").roles, roles(1, 1, 1, 1, 2));
}

#[test]
fn picking_the_suggested_set_again_makes_it_follow_the_player_count() {
    let (mut lobby, outputs) = lobby_of(6);
    let mut edited = settings_seen_by(&outputs, "P1");
    edited.roles = roles(1, 1, 1, 1, 2);
    let outputs = lobby
        .handle(
            &seat("P1"),
            Command::UpdateSettings {
                settings: edited.clone(),
            },
        )
        .unwrap();
    edited.roles = view(&outputs, "P1").suggested_roles;
    lobby
        .handle(&seat("P1"), Command::UpdateSettings { settings: edited })
        .unwrap();

    let outputs = lobby.handle(&seat("P7"), join("P7")).unwrap();

    assert_eq!(settings_seen_by(&outputs, "P1").roles, roles(2, 1, 1, 1, 2));
}

#[test]
fn settings_out_of_bounds_are_refused() {
    let (mut lobby, outputs) = lobby_of(6);
    let valid = settings_seen_by(&outputs, "P1");
    let mut too_many_roles = valid.clone();
    too_many_roles.roles.villager = 20;
    let mut instant_timer = valid.clone();
    instant_timer.timers.seer = 0;
    let mut endless_timer = valid.clone();
    endless_timer.timers.discussion = 24 * 3600;

    for settings in [too_many_roles, instant_timer, endless_timer] {
        assert_eq!(
            lobby.handle(&seat("P1"), Command::UpdateSettings { settings }),
            Err(Rejection::InvalidSettings)
        );
    }
}

fn join(name: &str) -> Command {
    Command::Join {
        name: name.to_string(),
    }
}

fn edit_roles(lobby: &mut Engine, roles: RoleCounts) -> Outputs {
    let host = lobby.handle(&seat("P1"), join("P1")).unwrap();
    let mut settings = settings_seen_by(&host, "P1");
    settings.roles = roles;
    lobby
        .handle(&seat("P1"), Command::UpdateSettings { settings })
        .unwrap()
}

#[test]
fn the_game_cannot_start_with_fewer_than_five_players() {
    let (mut lobby, outputs) = lobby_of(4);

    assert_eq!(
        view(&outputs, "P2").start_blocked_by,
        Some(Rejection::NotEnoughPlayers)
    );
    assert_eq!(
        lobby.handle(&seat("P1"), Command::Start),
        Err(Rejection::NotEnoughPlayers)
    );
}

#[test]
fn the_game_cannot_start_unless_there_are_as_many_roles_as_players() {
    let (mut lobby, _) = lobby_of(6);

    for mismatched in [roles(2, 1, 1, 1, 0), roles(2, 1, 1, 1, 2)] {
        let outputs = edit_roles(&mut lobby, mismatched);
        assert_eq!(
            view(&outputs, "P2").start_blocked_by,
            Some(Rejection::RoleCountMismatch)
        );
        assert_eq!(
            lobby.handle(&seat("P1"), Command::Start),
            Err(Rejection::RoleCountMismatch)
        );
    }
}

#[test]
fn the_game_can_start_with_five_to_twelve_players_and_matching_roles() {
    for count in [5, 12] {
        let (_, outputs) = lobby_of(count);
        assert_eq!(
            view(&outputs, "P1").start_blocked_by,
            None,
            "{count} Players"
        );
    }
}

#[test]
fn only_the_host_can_start_the_game() {
    let (mut lobby, _) = lobby_of(5);

    assert_eq!(
        lobby.handle(&seat("P2"), Command::Start),
        Err(Rejection::NotHost)
    );
}

/// Starts a Game of `count` Players with the suggested Roles.
fn started(count: usize, seed: u64) -> (Engine, Outputs) {
    let (mut lobby, _) = seeded_lobby_of(count, seed);
    let outputs = lobby.handle(&seat("P1"), Command::Start).unwrap();
    (lobby, outputs)
}

fn role_of(outputs: &Outputs, name: &str) -> Role {
    view(outputs, name)
        .role
        .as_ref()
        .unwrap_or_else(|| panic!("{name} has no Role"))
        .role
}

fn deal(outputs: &Outputs, count: usize) -> Vec<Role> {
    (1..=count).map(|n| role_of(outputs, &player(n))).collect()
}

#[test]
fn nobody_has_a_role_before_the_game_starts() {
    let (_, outputs) = lobby_of(5);

    assert!(view(&outputs, "P3").role.is_none());
}

#[test]
fn starting_deals_exactly_the_roles_of_the_settings() {
    let (_, outputs) = started(8, 7);

    let dealt = deal(&outputs, 8);
    let count = |role| dealt.iter().filter(|&&r| r == role).count();
    assert_eq!(count(Role::Werewolf), 2);
    assert_eq!(count(Role::Seer), 1);
    assert_eq!(count(Role::Witch), 1);
    assert_eq!(count(Role::Hunter), 1);
    assert_eq!(count(Role::Villager), 3);
}

#[test]
fn the_deal_comes_from_the_injected_randomness() {
    let (_, first) = started(8, 7);
    let (_, replay) = started(8, 7);
    assert_eq!(deal(&first, 8), deal(&replay, 8));

    let deals: HashSet<Vec<Role>> = (1..=20).map(|seed| deal(&started(8, seed).1, 8)).collect();
    assert!(deals.len() > 1, "every seed dealt the same Roles");
}

#[test]
fn each_role_card_shows_its_camp() {
    let (_, outputs) = started(12, 3);

    for n in 1..=12 {
        let card = view(&outputs, &player(n)).role.clone().unwrap();
        let expected = match card.role {
            Role::Werewolf => Camp::Werewolves,
            Role::Villager | Role::Seer | Role::Witch | Role::Hunter => Camp::Village,
        };
        assert_eq!(card.camp, expected, "{:?}", card.role);
    }
}

#[test]
fn werewolves_see_the_other_werewolves_and_nobody_else_sees_any_role() {
    let (_, outputs) = started(12, 5);
    let werewolves: HashSet<PlayerId> = (1..=12)
        .map(|n| view(&outputs, &player(n)))
        .filter(|v| v.role.as_ref().unwrap().role == Role::Werewolf)
        .map(|v| v.you)
        .collect();
    assert_eq!(werewolves.len(), 3);

    for n in 1..=12 {
        let view = view(&outputs, &player(n));
        let seen: HashSet<PlayerId> = view
            .role
            .as_ref()
            .unwrap()
            .fellow_werewolves
            .iter()
            .copied()
            .collect();
        if werewolves.contains(&view.you) {
            let others: HashSet<PlayerId> = werewolves
                .iter()
                .copied()
                .filter(|&w| w != view.you)
                .collect();
            assert_eq!(seen, others);
        } else {
            assert!(seen.is_empty());
        }
    }
}

#[test]
fn a_player_who_reconnects_sees_their_role_again() {
    let (mut lobby, outputs) = started(5, 9);
    let before = view(&outputs, "P4").role.clone();
    lobby.handle(&seat("P4"), Command::Disconnect).unwrap();

    let outputs = lobby.handle(&seat("P4"), join("P4")).unwrap();

    assert_eq!(view(&outputs, "P4").role, before);
}

#[test]
fn once_started_the_settings_and_the_seats_are_frozen() {
    let (mut lobby, outputs) = started(5, 9);
    let settings = settings_seen_by(&outputs, "P1");

    for (who, command) in [
        ("P1", Command::UpdateSettings { settings }),
        ("P1", Command::Start),
        ("P2", Command::Leave),
        ("Late", join("Late")),
    ] {
        assert_eq!(
            lobby.handle(&seat(who), command),
            Err(Rejection::GameStarted)
        );
    }
}
