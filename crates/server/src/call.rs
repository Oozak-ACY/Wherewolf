//! The Lobby's video call: LiveKit access for each seated Player, and the
//! server-side lever of ADR 0001 (turning a Player's `canSubscribe` off while
//! they may see and hear nobody).
//!
//! The game server is the only one holding the LiveKit keys, and they only
//! ever come from the environment (see `.env.example`).

use std::time::Duration;

use livekit_api::access_token::{AccessToken, VideoGrants};
use livekit_api::services::room::{RoomClient, UpdateParticipantOptions};
use livekit_protocol::ParticipantPermission;

use crate::protocol::CallTicket;

/// Longer than any evening of play.
const TICKET_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Where the LiveKit server is and the keys to sign its tokens.
pub struct CallConfig {
    url: String,
    api_key: String,
    api_secret: String,
    rooms: RoomClient,
}

impl CallConfig {
    /// Reads `LIVEKIT_URL`, `LIVEKIT_API_KEY` and `LIVEKIT_API_SECRET`.
    pub fn from_env() -> Option<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// `None` unless all three variables are filled in.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let var = |name| {
            var(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let (url, api_key, api_secret) = (
            var("LIVEKIT_URL")?,
            var("LIVEKIT_API_KEY")?,
            var("LIVEKIT_API_SECRET")?,
        );
        Some(Self {
            rooms: RoomClient::with_api_key(&url, &api_key, &api_secret),
            url,
            api_key,
            api_secret,
        })
    }

    /// Lets one Player into a room, seen by the others under `identity`, with
    /// their display name. `can_subscribe` is off while they may receive nobody.
    pub fn ticket(
        &self,
        room: &str,
        identity: &str,
        name: &str,
        can_subscribe: bool,
    ) -> CallTicket {
        let token = AccessToken::with_api_key(&self.api_key, &self.api_secret)
            .with_identity(identity)
            .with_name(name)
            .with_ttl(TICKET_TTL)
            .with_grants(VideoGrants {
                room_join: true,
                room: room.to_string(),
                can_publish: true,
                can_subscribe,
                // Players only talk to each other through the game server.
                can_publish_data: false,
                ..Default::default()
            })
            .to_jwt()
            .expect("a token with an identity and a room always signs");
        CallTicket {
            url: self.url.clone(),
            token,
        }
    }

    /// Switches whether a Player in the room may receive anyone at all.
    pub async fn set_can_subscribe(
        &self,
        room: &str,
        identity: &str,
        can_subscribe: bool,
    ) -> Result<(), String> {
        let options = UpdateParticipantOptions {
            permission: Some(permission(can_subscribe)),
            ..Default::default()
        };
        self.rooms
            .update_participant(room, identity, options)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Who is in the room right now, by identity, and whether each may subscribe.
    pub async fn subscribers(&self, room: &str) -> Result<Vec<(String, bool)>, String> {
        let participants = self
            .rooms
            .list_participants(room)
            .await
            .map_err(|e| e.to_string())?;
        Ok(participants
            .into_iter()
            .map(|p| {
                let can_subscribe = p.permission.is_some_and(|p| p.can_subscribe);
                (p.identity, can_subscribe)
            })
            .collect())
    }
}

/// A Player's full permission set. LiveKit replaces it as a whole, so the rest
/// must match the ticket: `canPublish` stays on, since revoking it would force
/// a republish at every Turn. The fields left to their defaults are ones the
/// ticket leaves at theirs too (sources, metadata, hidden, recorder, agent).
fn permission(can_subscribe: bool) -> ParticipantPermission {
    ParticipantPermission {
        can_subscribe,
        can_publish: true,
        can_publish_data: false,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use livekit_api::access_token::TokenVerifier;

    fn config() -> CallConfig {
        CallConfig::from_vars(|name| match name {
            "LIVEKIT_URL" => Some("wss://wherewolf.livekit.cloud".into()),
            "LIVEKIT_API_KEY" => Some("APIkey123".into()),
            "LIVEKIT_API_SECRET" => Some("a-secret-long-enough-to-sign-tokens".into()),
            _ => None,
        })
        .expect("all three variables are set")
    }

    #[test]
    fn a_players_ticket_lets_them_join_the_lobby_room_under_their_player_id_and_name() {
        let ticket = config().ticket("LOUPS-x7k2", "3", "Camille", true);

        assert_eq!(ticket.url, "wss://wherewolf.livekit.cloud");
        let claims =
            TokenVerifier::with_api_key("APIkey123", "a-secret-long-enough-to-sign-tokens")
                .verify(&ticket.token)
                .expect("signed with the configured key");
        assert_eq!(claims.sub, "3");
        assert_eq!(claims.name, "Camille");
        assert_eq!(claims.video.room, "LOUPS-x7k2");
        assert!(claims.video.room_join);
        assert!(claims.video.can_publish);
        assert!(claims.video.can_subscribe);
        assert!(!claims.video.can_publish_data);
        assert!(!claims.video.room_admin);
    }

    #[test]
    fn a_player_who_may_receive_nobody_joins_unable_to_subscribe() {
        let ticket = config().ticket("LOUPS-x7k2", "3", "Camille", false);

        let claims =
            TokenVerifier::with_api_key("APIkey123", "a-secret-long-enough-to-sign-tokens")
                .verify(&ticket.token)
                .unwrap();
        assert!(!claims.video.can_subscribe);
        assert!(claims.video.can_publish);
    }

    #[test]
    fn switching_subscriptions_never_revokes_publishing() {
        for can_subscribe in [true, false] {
            let permission = permission(can_subscribe);
            assert_eq!(permission.can_subscribe, can_subscribe);
            assert!(permission.can_publish);
            assert!(!permission.can_publish_data);
        }
    }

    #[test]
    fn the_call_is_unavailable_unless_every_livekit_variable_is_filled_in() {
        for missing in ["LIVEKIT_URL", "LIVEKIT_API_KEY", "LIVEKIT_API_SECRET"] {
            for absent in [None, Some(""), Some("   ")] {
                let config = CallConfig::from_vars(|name| {
                    if name == missing {
                        absent.map(String::from)
                    } else {
                        Some("set".into())
                    }
                });
                assert!(config.is_none(), "{missing} = {absent:?}");
            }
        }
    }
}
