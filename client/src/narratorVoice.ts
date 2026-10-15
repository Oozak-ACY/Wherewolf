// Plays the Narrator's cues on the Table device, one line after the other.

import type { Cue, FixedLine } from "./narratorScript";

export type NarratorVoiceDeps = {
  /**
   * Plays the MP3 of a fixed line, resolving once it ends or is stopped.
   * `null` when that MP3 is not there to play.
   */
  playMp3: (key: FixedLine) => Promise<void> | null;
  /** Speaks `text` with the browser's voice, resolving once it ends or is stopped. */
  speak: (text: string) => Promise<void>;
  /** Silences whatever line is playing. */
  stop: () => void;
};

/**
 * The Narrator's voice: fixed lines from their MP3, and live lines, or fixed
 * lines whose MP3 is missing, with browser speech. Sounds and backgrounds
 * are left to the ambiance mix.
 */
export function createNarratorVoice({ playMp3, speak, stop }: NarratorVoiceDeps) {
  // Bumped by each new set of cues, so the previous ones stop where they are.
  let latestCues = 0;

  const silence = () => {
    latestCues++;
    stop();
  };

  return {
    /** Plays `cues` in order, cutting whatever the previous ones were saying. */
    play(cues: Cue[]) {
      silence();
      const current = latestCues;
      void (async () => {
        for (const cue of cues) {
          if (cue.type === "fixed") await (playMp3(cue.key) ?? speak(cue.text));
          else if (cue.type === "live") await speak(cue.text);
          if (latestCues !== current) return;
        }
      })();
    },
    /** Silences the line playing, and drops the ones after it. */
    stop: silence,
  };
}
