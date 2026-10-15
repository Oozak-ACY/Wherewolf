//! The Narrator's live lines: the ones with names in them, spoken with the
//! same ElevenLabs voice as the fixed lines, generated as the Game goes.
//!
//! The game server is the only one holding the ElevenLabs key, and it only
//! ever comes from the environment (see `.env.example`). It is never logged
//! and never sent to a client.

use std::fmt;
use std::time::Duration;

use axum::body::Bytes;
use futures_util::{Stream, StreamExt};

/// "Narrateur sombre", made with ElevenLabs Voice Design.
pub const DEFAULT_VOICE_ID: &str = "4ocweN2jeAEzWpQYYKak";
/// Fast enough to start while the fixed line before it plays.
const MODEL: &str = "eleven_flash_v2_5";
/// Longer than any line the Narrator says: every Player tied, with the
/// longest names.
pub const MAX_LINE_CHARS: usize = 500;
/// ElevenLabs starts sending audio after about 1.5 s.
const TIMEOUT: Duration = Duration::from_secs(15);

/// The voice that says the live lines, and the key to use it.
pub struct Voice {
    api_key: String,
    voice_id: String,
    http: reqwest::Client,
}

/// Never shows the key.
impl fmt::Debug for Voice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Voice")
            .field("voice_id", &self.voice_id)
            .finish_non_exhaustive()
    }
}

impl Voice {
    /// Reads `ELEVENLABS_API_KEY`, and `ELEVENLABS_VOICE_ID` if set.
    pub fn from_env() -> Option<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// `None` without a key. The voice is the Narrator's unless another is
    /// filled in.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let var = |name| {
            var(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Some(Self {
            api_key: var("ELEVENLABS_API_KEY")?,
            voice_id: var("ELEVENLABS_VOICE_ID").unwrap_or_else(|| DEFAULT_VOICE_ID.into()),
            http: reqwest::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .expect("the HTTP client builds"),
        })
    }

    /// Asks ElevenLabs to say `text`, and returns its MP3 as it arrives.
    pub async fn speak(
        &self,
        text: &str,
    ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>> + use<>, String> {
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}/stream?output_format=mp3_44100_128",
            self.voice_id
        );
        let response = self
            .http
            .post(url)
            .header("xi-api-key", &self.api_key)
            .json(&serde_json::json!({ "text": text, "model_id": MODEL }))
            .send()
            .await
            // Without the URL, which names the voice: the message stays short.
            .map_err(|e| e.without_url().to_string())?;
        if !response.status().is_success() {
            return Err(format!("ElevenLabs answered {}", response.status()));
        }
        Ok(response.bytes_stream())
    }
}

/// Why a live line was refused before reaching the voice.
#[derive(Debug, PartialEq, Eq)]
pub enum LineRefusal {
    Blank,
    TooLong,
}

/// A line the Narrator could say: not blank, and no longer than any of his.
pub fn check_line(text: &str) -> Result<(), LineRefusal> {
    if text.trim().is_empty() {
        Err(LineRefusal::Blank)
    } else if text.chars().count() > MAX_LINE_CHARS {
        Err(LineRefusal::TooLong)
    } else {
        Ok(())
    }
}

/// Passes `audio` on as it arrives, and hands the whole of it to `heard`
/// once it has all arrived. An error on the way leaves `heard` uncalled, so a
/// broken line is never kept.
pub fn recording<E>(
    audio: impl Stream<Item = Result<Bytes, E>> + Unpin,
    heard: impl FnOnce(Bytes),
) -> impl Stream<Item = Result<Bytes, E>> {
    futures_util::stream::unfold(
        (audio, Vec::new(), Some(heard)),
        |(mut audio, mut so_far, heard)| async move {
            match audio.next().await {
                Some(Ok(chunk)) => {
                    so_far.extend_from_slice(&chunk);
                    Some((Ok(chunk), (audio, so_far, heard)))
                }
                Some(Err(e)) => Some((Err(e), (audio, so_far, None))),
                None => {
                    if let Some(heard) = heard {
                        heard(Bytes::from(so_far));
                    }
                    None
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn voice(vars: &[(&str, &str)]) -> Option<Voice> {
        Voice::from_vars(|name| {
            vars.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn live_lines_are_unavailable_without_a_key() {
        for absent in [None, Some(""), Some("   ")] {
            let vars: Vec<_> = absent
                .map(|v| ("ELEVENLABS_API_KEY", v))
                .into_iter()
                .collect();
            assert!(voice(&vars).is_none(), "{absent:?}");
        }
    }

    #[test]
    fn the_narrators_voice_unless_another_is_configured() {
        let narrator = voice(&[("ELEVENLABS_API_KEY", "sk_secret")]).unwrap();
        assert_eq!(narrator.voice_id, DEFAULT_VOICE_ID);

        let other = voice(&[
            ("ELEVENLABS_API_KEY", "sk_secret"),
            ("ELEVENLABS_VOICE_ID", "another"),
        ])
        .unwrap();
        assert_eq!(other.voice_id, "another");
    }

    #[test]
    fn the_key_never_shows_in_logs() {
        let voice = voice(&[("ELEVENLABS_API_KEY", "sk_secret")]).unwrap();

        assert!(!format!("{voice:?}").contains("sk_secret"));
    }

    #[test]
    fn a_line_is_neither_blank_nor_longer_than_any_of_the_narrators() {
        assert_eq!(
            check_line("Cette nuit, le village a perdu Camille."),
            Ok(())
        );
        assert_eq!(check_line(&"é".repeat(MAX_LINE_CHARS)), Ok(()));
        assert_eq!(
            check_line(&"é".repeat(MAX_LINE_CHARS + 1)),
            Err(LineRefusal::TooLong)
        );
        assert_eq!(check_line("  "), Err(LineRefusal::Blank));
    }

    async fn played<E>(audio: impl Stream<Item = Result<Bytes, E>>) -> Vec<Result<Bytes, E>> {
        audio.collect().await
    }

    #[tokio::test]
    async fn a_line_heard_whole_is_kept_whole() {
        let kept = Rc::new(RefCell::new(None));
        let keep = kept.clone();
        let chunks = [Ok::<_, ()>(Bytes::from("ID3")), Ok(Bytes::from("mp3"))];

        let passed = played(recording(
            futures_util::stream::iter(chunks),
            move |audio| *keep.borrow_mut() = Some(audio),
        ))
        .await;

        assert_eq!(passed, vec![Ok(Bytes::from("ID3")), Ok(Bytes::from("mp3"))]);
        assert_eq!(*kept.borrow(), Some(Bytes::from("ID3mp3")));
    }

    #[tokio::test]
    async fn a_line_broken_on_the_way_is_never_kept() {
        let kept = Rc::new(RefCell::new(false));
        let keep = kept.clone();
        let chunks = [Ok(Bytes::from("ID3")), Err("cut"), Ok(Bytes::from("mp3"))];

        let passed = played(recording(futures_util::stream::iter(chunks), move |_| {
            *keep.borrow_mut() = true
        }))
        .await;

        assert_eq!(passed[1], Err("cut"));
        assert!(!*kept.borrow());
    }
}
