# PROTOTYPE: call + Narrator voice (throwaway, do not merge to main)

Answers the two questions in [`docs/handoffs/prototype-call-and-voice.md`](../../docs/handoffs/prototype-call-and-voice.md).
The "game server" is a Vite middleware in `vite.config.js` (Node, not Rust): the question is about phones, not the server language.
State lives in memory, and restarting the server resets it to Day.

## Run

1. Create a free LiveKit Cloud project. Copy `.env.example` to `.env` (git-ignored) and paste in the URL, key and secret.
2. `npm install` (once), then `npm run proto`.
3. In a second terminal, run `npm run tunnel` (ngrok) and open the `https://…ngrok…` URL on every phone. Phones need HTTPS for camera and mic.

## Test script (3+ devices; ideally an iPhone, an Android and a laptop)

Join as **2 Werewolves and 1+ Villagers**. Anyone can press the Turn buttons; it's a test harness.

**Q1: Narrator voice during a live call**
- Tap **🗣 Narrator** on the iPhone. Is it audible? Loudspeaker or earpiece? Is it cut, or does call audio duck?
- Press **🐺 / ☀️** from *another* device. Does the iPhone speak the line **without being tapped**? (The log shows `speech START`. If only `speak() called` appears, the voice was blocked.)
- Rejoin with **"Unlock voice…" unchecked** to see whether the `speak("")` unlock matters.
- Untick **"mute mic while Narrator speaks"** and press Narrator. Do the *other* devices hear the voice echo through this phone's mic?
- Surprise knob: the **audioSession** dropdown (iOS 17+ Safari) changes how iOS routes audio.

**Q2: switching visibility per Turn**
- Press **🐺 Werewolves' Turn**. Villagers should black out and lose all audio, and the Wolves should see only each other. Press **☀️ Day**. Does everyone come back?
- Latency: the log times everything as `+Nms` after the Turn message arrives (`subscribed`, `unsubscribed`, `first frame`), and the presser's log shows the server-side cost.
- If Day doesn't restore someone, try **↻ Resubscribe** and note it.
- Tap **📋 Copy log** on each device to paste the results back.

## Bring back

## Results: session of 2026-09-24 (4 players, 2 Werewolves)

| Device + browser | Voice audible? | Works without a tap? | Unlock needed? | Echo when mic on? | Switch latency | Back to Day OK? |
|---|---|---|---|---|---|---|
| Android 17, Firefox 156 (Loan) | yes, voice "français (France)" | yes | ticked, OK | not tested | ~270–475 ms to drop, ~280–440 ms to restore | yes, every time |
| iPhone iOS 18.7, Safari 26.6 (Anhhh) | yes, voice "Thomas" | yes | ticked, OK | not tested | ~210–450 ms | yes |
| iPhone iOS 18.1.1, Safari 18.1 (Hitman) | yes, voice "Grandma" (!) | yes | ticked, OK | not tested | ~190–320 ms | yes |
| iPhone iOS 18.7, Safari 26.6 (Nozzzz) | **no, never starts** | n/a | **unticked → silent** | not tested | ~270 ms | yes |

**Verdict**
- **Q1, voice: OK, on one condition.** `speechSynthesis` in `fr-FR` plays during a live LiveKit call on iOS Safari and Android Firefox. Later lines play without a tap **only if** a `speak("")` runs inside the Join tap. Without it, iOS stays silent and never fires `onstart`/`onend`. The on-screen text fallback is not needed.
- **Q2, Turn switch: OK.** Both ADR 0001 levers take effect in ~200–450 ms. The first video frame follows 20–200 ms later, with no visible glitch. Day restored cleanly every time, and "Resubscribe" was never needed.

**Surprises to carry into the real client**
- iOS returns the *first* `fr-FR` voice, which can be a novelty voice ("Grandma"). Pick the voice explicitly: a preferred list, excluding the novelty voices.
- When the voice is blocked, `onend` never fires. Anything waiting on it (re-enabling the mic) needs a timeout, or the mic stays muted forever. This likely explains the "mics don't work" report from the first session.
- Every mic is muted for 2–4 s per Narrator line, which clips the start of Day discussion.
- Remote `<audio>` elements from the harness are not all cleaned up after resubscribe cycles. This is a bug in the prototype, not a LiveKit finding.
- Not tested: echo of the Narrator into other players' audio when mics stay on.
