// Plays the Narrator's cues on the Table device, one line after the other.

import type { Cue, FixedLine } from "./narratorScript";

/**
 * How long a live line may keep the table waiting, once its turn comes,
 * before browser speech says it instead.
 */
export const LIVE_LINE_PATIENCE_MS = 3000;

export type NarratorVoiceDeps = {
  /**
   * Plays the MP3 of a fixed line, resolving once it ends or is stopped.
   * `null` when that MP3 is not there to play.
   */
  playMp3: (key: FixedLine) => Promise<void> | null;
  /** Speaks `text` with the browser's voice, resolving once it ends or is stopped. */
  speak: (text: string) => Promise<void>;
  /** Has the server voice a live line, resolving to whether its audio arrived. */
  loadLive: (text: string) => Promise<boolean>;
  /**
   * Plays a live line's audio once it arrived, resolving once it ends or is
   * stopped. `null` when it can't play.
   */
  playLive: (text: string) => Promise<void> | null;
  /** Silences whatever line is playing. */
  stop: () => void;
};

/**
 * The Narrator's voice: fixed lines from their MP3, live lines as the server
 * voiced them, and browser speech for a fixed line whose MP3 is missing or a
 * live line that failed or is late. Sounds and backgrounds are left to the
 * ambiance mix.
 */
export function createNarratorVoice({ playMp3, speak, loadLive, playLive, stop }: NarratorVoiceDeps) {
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
      // Requested right away, so they are voiced while the lines before them play.
      const loading = new Map(
        cues.flatMap((cue) => (cue.type === "live" ? [[cue.text, loadLive(cue.text).catch(() => false)]] : [])),
      );
      void (async () => {
        for (const cue of cues) {
          if (cue.type === "fixed") await (playMp3(cue.key) ?? speak(cue.text));
          else if (cue.type === "live") {
            const arrived = await withinPatience(loading.get(cue.text)!);
            if (latestCues !== current) return;
            await ((arrived && playLive(cue.text)) || speak(cue.text));
          }
          if (latestCues !== current) return;
        }
      })();
    },
    /** Silences the line playing, and drops the ones after it. */
    stop: silence,
  };
}

/** Whether a live line arrived, giving up after `LIVE_LINE_PATIENCE_MS`. */
function withinPatience(arrival: Promise<boolean>): Promise<boolean> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<boolean>((resolve) => {
    timer = setTimeout(() => resolve(false), LIVE_LINE_PATIENCE_MS);
  });
  return Promise.race([arrival, late]).finally(() => clearTimeout(timer));
}
