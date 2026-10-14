# In-person play first; Remote play deferred

Wherewolf was designed for friends playing remotely over an in-app voice and video call. We are pivoting to a game played around one physical table first: the Players debate and vote out loud as in real-life Werewolf, the Narrator's voice comes from the Host's phone on loudspeaker (the Table device), and each Player confirms their secret actions (the Werewolves' Victim, the Seer's inspection, the Witch's potions, the Vote) on their own phone, which stays dark and silent. The machine's job shrinks to what a human moderator does badly at a table: keeping secrets, timing, and announcing outcomes without bias. The Game rules engine is unchanged; only the medium changes.

Remote play is not abandoned: it comes back later as a second Mode chosen in the Settings. Its code (the call, LiveKit media, the Visibility plan) stays in the repository but is not reachable from the in-person game.

## Considered options

- **Ship both Modes now:** rejected — splits effort before either is good, and the remote call is the hardest, least validated part (real-phone issues still open).
- **Every phone speaks the Narrator's lines:** rejected — eight phones speaking a split second apart, lighting up while eyes are closed, gives Players away.
- **A separate non-playing Table device (tablet/laptop):** deferred — the Host's phone needs no extra hardware; a dedicated Table page can come later.

## Consequences

- [ADR 0001](0001-server-enforced-media-visibility.md) applies only to the future Remote Mode; the in-person game needs no media server.
- The Host's phone is a single point of failure for the voice, so the Game is Paused while the Host is Disconnected.
- Night Turns end on confirmation (with the timer as a backstop) instead of on the timer alone, except where Hidden Roles requires a dead Role's Turn to run its full time.
- Glossary terms specific to the call are marked _Remote only_ in `CONTEXT.md`.
- The Narrator's voice moves from browser speech synthesis to ElevenLabs, over a sound mix played by the Table device: see [ADR 0004](0004-narrator-voice-and-ambiance.md).
