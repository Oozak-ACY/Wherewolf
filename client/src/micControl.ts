// Whether this Player's published microphone is live: they want to talk, and
// the Narrator is not speaking. Each side only ever moves its own half.

/** How this Player talks: hold a button, or an open mic they mute themselves. */
export type MicMode = "pushToTalk" | "open";

/** The Narrator's half of the mic: muted while one of its lines plays. */
export type NarratorSpeaking = { started(): void; ended(): void };

/**
 * `apply` runs each time the mic goes live or muted afterwards, to mute or
 * unmute the published track. The mic starts muted in push-to-talk, live in open mic.
 */
export function createMicControl(initialMode: MicMode, apply: (live: boolean) => void) {
  let mode = initialMode;
  // Push-to-talk: the button or Space is held. Open mic: not muted by the Player.
  let wantsToTalk = mode === "open";
  let narrating = false;
  // No track exists yet: whoever publishes it reads `live()` first.
  let live = wantsToTalk;

  const update = () => {
    const next = wantsToTalk && !narrating;
    if (next === live) return;
    live = next;
    apply(live);
  };

  return {
    mode: () => mode,
    live: () => live,
    /** The Player's half: holding to talk, or not muted in open mic. */
    wantsToTalk: () => wantsToTalk,
    setMode(next: MicMode) {
      mode = next;
      // Open mic starts live; push-to-talk starts released.
      wantsToTalk = mode === "open";
      update();
    },
    /** The button or Space went down. Push-to-talk only. */
    press() {
      if (mode !== "pushToTalk") return;
      wantsToTalk = true;
      update();
    },
    /** The button or Space went up, or the page lost focus. Push-to-talk only. */
    release() {
      if (mode !== "pushToTalk") return;
      wantsToTalk = false;
      update();
    },
    /** A tap on the open mic's button. */
    toggle() {
      if (mode !== "open") return;
      wantsToTalk = !wantsToTalk;
      update();
    },
    narratorStarted() {
      narrating = true;
      update();
    },
    narratorEnded() {
      narrating = false;
      update();
    },
  };
}
