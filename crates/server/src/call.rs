//! The Lobby's video call: LiveKit access for each seated Player.
//!
//! The game server is the only one holding the LiveKit keys, and they only
//! ever come from the environment (see `.env.example`).

use std::time::Duration;

use livekit_api::access_token::{AccessToken, VideoGrants};

use crate::protocol::CallTicket;

/// Longer than any evening of play.
const TICKET_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Where the LiveKit server is and the keys to sign its tokens.
pub struct CallConfig {
    url: String,
    api_key: String,
    api_secret: String,
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
        Some(Self {
            url: var("LIVEKIT_URL")?,
            api_key: var("LIVEKIT_API_KEY")?,
            api_secret: var("LIVEKIT_API_SECRET")?,
        })
    }

    /// Lets one Player into a room, seen by the others under `identity`, with
    /// their display name.
    pub fn ticket(&self, room: &str, identity: &str, name: &str) -> CallTicket {
        let token = AccessToken::with_api_key(&self.api_key, &self.api_secret)
            .with_identity(identity)
            .with_name(name)
            .with_ttl(TICKET_TTL)
            .with_grants(VideoGrants {
                room_join: true,
                room: room.to_string(),
                can_publish: Some(true),
                can_subscribe: Some(true),
                // Players only talk to each other through the game server.
                can_publish_data: Some(false),
                ..Default::default()
            })
            .to_jwt()
            .expect("a token with an identity and a room always signs");
        CallTicket {
            url: self.url.clone(),
            token,
        }
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
        let ticket = config().ticket("LOUPS-x7k2", "3", "Camille");

        assert_eq!(ticket.url, "wss://wherewolf.livekit.cloud");
        let claims =
            TokenVerifier::with_api_key("APIkey123", "a-secret-long-enough-to-sign-tokens")
                .verify(&ticket.token)
                .expect("signed with the configured key");
        assert_eq!(claims.sub, "3");
        assert_eq!(claims.name, "Camille");
        assert_eq!(claims.video.room, "LOUPS-x7k2");
        assert!(claims.video.room_join);
        assert!(claims.video.can_publish());
        assert!(claims.video.can_subscribe());
        assert!(!claims.video.can_publish_data());
        assert!(!claims.video.room_admin);
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
