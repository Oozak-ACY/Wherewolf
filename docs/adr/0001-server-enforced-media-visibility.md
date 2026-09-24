# Who-sees-whom is enforced by the media server, not hidden on screen

Players' cameras and mics are live in-app, and the Narrator changes who may see and hear whom at every Turn (e.g. only Werewolves during their Turn, dead Players as Spectators). All audio/video goes through a media server (SFU, LiveKit), and a device only ever receives the streams it is allowed to — we never send everything and hide it in the UI, because anyone could open the browser dev tools and listen to the Werewolves. A peer-to-peer mesh was also rejected: 12 phones each sending video to 11 others does not hold up on mobile.

LiveKit has no server-side "X may not receive Y" rule, so enforcement is built from two levers:

1. **Hard, server-side:** the game server turns a Player's `canSubscribe` off whenever they must perceive nobody (e.g. sleeping Players at Night). That device then receives nothing, whatever its code does.
2. **Publisher-side, SFU-enforced:** each client restricts who may subscribe to its own tracks (`setTrackSubscriptionPermissions`), following instructions from the game server.

The accepted residual risk: a tampered client can only broadcast **its own** stream to people who shouldn't hear it (e.g. a Spectator talking to the living) — the equivalent of shouting through the door, not eavesdropping.

## Consequences

- The game server drives both levers at every Turn change; visibility is a Narrator decision, never a client one.
- Revoking `canPublish` forces a republish, so it's avoided for routine Turn changes.
- Running the game needs a media server: LiveKit Cloud's free plan for now (enough for occasional sessions, hard-capped), self-hosting later if usage grows — only the server URL and keys change.
