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

| Device + browser | Voice audible? | Route / ducking | Works without a tap? | Unlock needed? | Echo when mic on? | Switch latency to Wolves | Back to Day OK? | Surprises |
|---|---|---|---|---|---|---|---|---|
| | | | | | | | | |

Agreed fallback if the voice fails on iOS: on-screen text + sound cues only.
