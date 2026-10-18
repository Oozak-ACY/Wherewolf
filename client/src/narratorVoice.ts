// Plays the Narrator's cues on the Table device, one line after the other.

import type { Cue, FixedLine, Phase, Sound } from "./narratorScript";

/**
 * How long a live line may keep the table waiting, once its turn comes,
 * before browser speech says it instead.
 */
export const LIVE_LINE_PATIENCE_MS = 3000;

/** The silence after each of the Narrator's lines, so each one lands. */
export const LINE_GAP_MS = 2000;

/** How long each sound plays before the line after it starts, over its tail. */
export const SOUND_LEAD_MS: Record<Sound, number> = {
  howl: 1800,
  bottle: 900,
  sword: 700,
  gunshot: 900,
  rooster: 1800,
};

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
  /** Plays a one-off sound, returning whether it could. */
  playSound: (sound: Sound) => boolean;
  /** Crossfades to the Phase's looped background, or out to silence. */
  setBackground: (phase: Phase | null) => void;
  /** Dips the background under the voice, or brings it back up. */
  duck: (on: boolean) => void;
};

/**
 * The Narrator's voice: fixed lines from their MP3, live lines as the server
 * voiced them, and browser speech for a fixed line whose MP3 is missing or a
 * live line that failed or is late. The ambiance follows the cues: each
 * background and sound in its place, the background dipping while lines play.
 */
export function createNarratorVoice({
  playMp3,
  speak,
  loadLive,
  playLive,
  stop,
  playSound,
  setBackground,
  duck,
}: NarratorVoiceDeps) {
  // Bumped by each new set of cues, so the previous ones stop where they are.
  let latestCues = 0;
  // The last background the cues playing have yet to change to. Cut cues
  // still change to it: the next ones may not, being in the same Phase.
  let owedBackground: Extract<Cue, { type: "background" }> | null = null;

  const silence = () => {
    if (owedBackground) setBackground(owedBackground.phase);
    owedBackground = null;
    latestCues++;
    stop();
  };

  return {
    /**
     * Plays `cues` in order, cutting whatever the previous ones were saying,
     * then calls `announced` once their last line has ended (at once if they
     * have none). Never called for cues cut before their end.
     */
    play(cues: Cue[], announced?: () => void) {
      silence();
      const current = latestCues;
      owedBackground = cues.filter((cue) => cue.type === "background").at(-1) ?? null;
      // Requested right away, so they are voiced while the lines before them play.
      const loading = new Map(
        cues.flatMap((cue) => (cue.type === "live" ? [[cue.text, loadLive(cue.text).catch(() => false)]] : [])),
      );
      void (async () => {
        let afterLine = false;
        for (const cue of cues) {
          if (afterLine) {
            await wait(LINE_GAP_MS);
            if (latestCues !== current) return;
          }
          afterLine = cue.type === "fixed" || cue.type === "live";
          if (cue.type === "background") {
            setBackground(cue.phase);
            if (cue === owedBackground) owedBackground = null;
          } else if (cue.type === "sound") {
            if (playSound(cue.sound)) await wait(SOUND_LEAD_MS[cue.sound]);
          } else {
            duck(true);
            if (cue.type === "fixed") await (playMp3(cue.key) ?? speak(cue.text));
            else {
              const arrived = await withinPatience(loading.get(cue.text)!);
              if (latestCues !== current) return;
              await ((arrived && playLive(cue.text)) || speak(cue.text));
            }
          }
          if (latestCues !== current) return;
        }
        duck(false);
        announced?.();
      })();
    },
    /** Silences the line playing, and drops the cues after it. */
    stop() {
      silence();
      duck(false);
    },
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

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
