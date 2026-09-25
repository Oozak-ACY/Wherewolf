//! Lobby scripts, driven only through the engine's public commands and outputs.

use wherewolf_engine::{
    Command, Engine, LobbyCode, Outputs, PlayerView, Randomness, Rejection, SeatToken,
};

/// The Lobby never draws at random.
struct NoRandomness;

impl Randomness for NoRandomness {
    fn below(&mut self, _: usize) -> usize {
        unreachable!("the Lobby draws nothing at random")
    }
}

fn lobby() -> Engine {
    Engine::new(LobbyCode::new("LOUPS"), NoRandomness)
}

fn seat(name: &str) -> SeatToken {
    SeatToken::new(format!("token-{name}"))
}

fn join(name: &str) -> Command {
    Command::Join {
        name: name.to_string(),
    }
}

fn view<'a>(outputs: &'a Outputs, name: &str) -> &'a PlayerView {
    outputs
        .view_for(&seat(name))
        .unwrap_or_else(|| panic!("no view for {name}"))
}

fn names(view: &PlayerView) -> Vec<&str> {
    view.players.iter().map(|p| p.name.as_str()).collect()
}

fn host_name(view: &PlayerView) -> &str {
    let host = view
        .players
        .iter()
        .find(|p| p.id == view.host)
        .expect("host is seated");
    &host.name
}

#[test]
fn every_player_sees_who_joined_and_who_the_host_is() {
    let mut lobby = lobby();
    lobby.handle(&seat("Alice"), join("Alice")).unwrap();
    let outputs = lobby.handle(&seat("Bob"), join("Bob")).unwrap();

    for name in ["Alice", "Bob"] {
        let view = view(&outputs, name);
        assert_eq!(view.code.as_str(), "LOUPS");
        assert_eq!(names(view), ["Alice", "Bob"]);
        assert_eq!(host_name(view), "Alice");
    }
    assert_eq!(names_of_you(&outputs, "Bob"), "Bob");
}

fn names_of_you<'a>(outputs: &'a Outputs, name: &str) -> &'a str {
    let view = view(outputs, name);
    &view.players.iter().find(|p| p.id == view.you).unwrap().name
}

#[test]
fn a_thirteenth_player_is_refused_and_nobody_else_is_affected() {
    let mut lobby = lobby();
    let twelve: Vec<String> = (1..=12).map(|n| format!("P{n}")).collect();
    for name in &twelve {
        lobby.handle(&seat(name), join(name)).unwrap();
    }

    let refused = lobby.handle(&seat("Late"), join("Late"));
    assert_eq!(refused, Err(Rejection::LobbyFull));

    let outputs = lobby.handle(&seat("P1"), join("P1")).unwrap();
    assert_eq!(names(view(&outputs, "P1")).len(), 12);
    assert!(outputs.view_for(&seat("Late")).is_none());
}

#[test]
fn when_the_host_leaves_the_next_player_to_have_joined_becomes_host() {
    let mut lobby = lobby();
    for name in ["Alice", "Bob", "Carol"] {
        lobby.handle(&seat(name), join(name)).unwrap();
    }

    let outputs = lobby.handle(&seat("Alice"), Command::Leave).unwrap();

    assert!(outputs.view_for(&seat("Alice")).is_none());
    for name in ["Bob", "Carol"] {
        let view = view(&outputs, name);
        assert_eq!(names(view), ["Bob", "Carol"]);
        assert_eq!(host_name(view), "Bob");
    }
}

#[test]
fn when_another_player_leaves_the_host_stays() {
    let mut lobby = lobby();
    for name in ["Alice", "Bob", "Carol"] {
        lobby.handle(&seat(name), join(name)).unwrap();
    }

    let outputs = lobby.handle(&seat("Bob"), Command::Leave).unwrap();

    let view = view(&outputs, "Carol");
    assert_eq!(names(view), ["Alice", "Carol"]);
    assert_eq!(host_name(view), "Alice");
}

#[test]
fn a_seat_freed_by_leaving_can_be_taken_by_someone_else() {
    let mut lobby = lobby();
    let twelve: Vec<String> = (1..=12).map(|n| format!("P{n}")).collect();
    for name in &twelve {
        lobby.handle(&seat(name), join(name)).unwrap();
    }
    lobby.handle(&seat("P5"), Command::Leave).unwrap();

    let outputs = lobby.handle(&seat("Late"), join("Late")).unwrap();

    assert_eq!(names(view(&outputs, "Late")).last(), Some(&"Late"));
}

#[test]
fn the_last_player_leaving_empties_the_lobby() {
    let mut lobby = lobby();
    lobby.handle(&seat("Alice"), join("Alice")).unwrap();

    let outputs = lobby.handle(&seat("Alice"), Command::Leave).unwrap();

    assert_eq!(outputs.views().count(), 0);
    assert!(lobby.is_empty());
}

#[test]
fn leaving_without_a_seat_is_refused() {
    let mut lobby = lobby();
    lobby.handle(&seat("Alice"), join("Alice")).unwrap();

    assert_eq!(
        lobby.handle(&seat("Bob"), Command::Leave),
        Err(Rejection::NotSeated)
    );
}

fn connected(view: &PlayerView, name: &str) -> bool {
    view.players
        .iter()
        .find(|p| p.name == name)
        .expect("seated")
        .connected
}

#[test]
fn a_disconnected_player_keeps_their_seat_and_others_see_them_offline() {
    let mut lobby = lobby();
    for name in ["Alice", "Bob"] {
        lobby.handle(&seat(name), join(name)).unwrap();
    }

    let outputs = lobby.handle(&seat("Bob"), Command::Disconnect).unwrap();

    let view = view(&outputs, "Alice");
    assert_eq!(names(view), ["Alice", "Bob"]);
    assert!(connected(view, "Alice"));
    assert!(!connected(view, "Bob"));
}

#[test]
fn reopening_the_link_with_the_same_seat_token_reclaims_the_same_seat() {
    let mut lobby = lobby();
    for name in ["Alice", "Bob", "Carol"] {
        lobby.handle(&seat(name), join(name)).unwrap();
    }
    let before = lobby.handle(&seat("Bob"), Command::Disconnect).unwrap();
    let bob_id = view(&before, "Bob").you;

    let after = lobby.handle(&seat("Bob"), join("Bob")).unwrap();

    let bob = view(&after, "Bob");
    assert_eq!(bob.you, bob_id);
    assert_eq!(names(bob), ["Alice", "Bob", "Carol"]);
    assert!(connected(view(&after, "Carol"), "Bob"));
}

#[test]
fn a_host_who_disconnects_is_still_host_when_they_come_back() {
    let mut lobby = lobby();
    for name in ["Alice", "Bob"] {
        lobby.handle(&seat(name), join(name)).unwrap();
    }

    let outputs = lobby.handle(&seat("Alice"), Command::Disconnect).unwrap();
    assert_eq!(host_name(view(&outputs, "Bob")), "Alice");

    let outputs = lobby.handle(&seat("Alice"), join("Alice")).unwrap();
    assert_eq!(host_name(view(&outputs, "Bob")), "Alice");
}

#[test]
fn a_player_can_reclaim_their_seat_in_a_full_lobby() {
    let mut lobby = lobby();
    let twelve: Vec<String> = (1..=12).map(|n| format!("P{n}")).collect();
    for name in &twelve {
        lobby.handle(&seat(name), join(name)).unwrap();
    }
    lobby.handle(&seat("P7"), Command::Disconnect).unwrap();

    let outputs = lobby.handle(&seat("P7"), join("P7")).unwrap();

    assert!(connected(view(&outputs, "P1"), "P7"));
}

#[test]
fn reclaiming_a_seat_with_a_new_name_renames_the_player() {
    let mut lobby = lobby();
    for name in ["Alice", "Bob"] {
        lobby.handle(&seat(name), join(name)).unwrap();
    }

    let outputs = lobby.handle(&seat("Bob"), join("Robert")).unwrap();

    assert_eq!(names(view(&outputs, "Alice")), ["Alice", "Robert"]);
}

#[test]
fn a_display_name_is_trimmed() {
    let mut lobby = lobby();

    let outputs = lobby.handle(&seat("Alice"), join("  Alice ")).unwrap();

    assert_eq!(names(view(&outputs, "Alice")), ["Alice"]);
}

#[test]
fn a_blank_or_overlong_display_name_is_refused() {
    let mut lobby = lobby();

    for name in ["", "   ", "Un nom beaucoup trop long pour une tuile"] {
        assert_eq!(
            lobby.handle(&seat("Alice"), join(name)),
            Err(Rejection::InvalidName)
        );
    }
    assert!(lobby.is_empty());
}
