# Handoff: prototype the call + Narrator voice on real phones

Run `/prototype` with this file. Build throwaway code on a `prototype/call-and-voice` branch; bring the answers back to the design session (which continues with `/to-spec`).

## Read first

- `CONTEXT.md`: glossary (Narrator, Turn, Spectator, …). Use these words.
- `docs/adr/0001-server-enforced-media-visibility.md`: how who-sees-whom is enforced.
- `docs/adr/0002-rust-game-server.md`: Rust server + TypeScript/React client.

## What Wherewolf is (one paragraph)

A French Werewolf game in the mobile browser for 5–12 friends, with an in-app LiveKit video/voice call. An automated **Narrator** runs the Game: at each **Turn** it changes who can see and hear whom (Day: all living see each other; Werewolves' Turn: only Werewolves see each other; everyone else blacked out and muted) and reads its lines aloud in French through the browser's `speechSynthesis`, with mics muted while it speaks.

## The two questions to answer

1. **Does the Narrator's French voice work on an iPhone during a live call?** With the mic active in a LiveKit room, does `speechSynthesis.speak()` (lang `fr-FR`) play audibly on iOS Safari, and on Android Chrome? Is it cut, rerouted to the earpiece, ducked, or blocked? Does unlocking it with a `speak("")` on the initial "Join" tap keep later lines working without a tap? Does it echo into other players' audio when mics are *not* muted?
2. **Does switching visibility at each Turn feel smooth?** Toggling between a "Day" state and a "Werewolves' Turn" state using the ADR 0001 levers: server-side `canSubscribe` on/off (via `livekit-api` `update_participant`) and client-side `setTrackSubscriptionPermissions`. How long until the switch takes effect, is there a visible or audible glitch, and does the client recover cleanly back to Day?

## Suggested shape (throwaway, keep it small)

- A single React page: join a room with a name, see a grid of tiles, a "Narrator" button that speaks "Le village s'endort…" and mutes the mic while speaking.
- A tiny server (Rust + axum + `livekit-api` if quick, otherwise whatever gets the answer fastest: the question is about phones, not the server language) that mints tokens and exposes "switch to Day" / "switch to Werewolves' Turn" with 2–3 hard-coded werewolf identities.
- LiveKit Cloud free project. **The user must create the account and paste the API key/secret into a local `.env`**: never commit it.
- Needs HTTPS to access camera/mic on phones (e.g. a tunnel or a quick Vercel deploy for the page).

## Bring back

A short note: per device tested (model + browser), yes/no for each question, the switch latency observed, and any surprises. If the voice fails on iOS, the agreed fallback is on-screen text + sound cues only.
