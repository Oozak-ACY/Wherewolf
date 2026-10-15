// The Table device's sound mix, with Web Audio. Three buses, Narrator, Music
// & ambiance and Sound effects, end in one limiter, so a boosted level never
// clips. The ambiance bus dips while the Narrator speaks.

import type { Phase, Sound } from "./narratorScript";

/** A sound file of the ambiance, in `public/ambiance` (see its `CREDITS.md`). */
export type AmbianceFile =
  | "night-music"
  | "night-wind"
  | "day-tavern"
  | "wolf-howl"
  | "bottle-smash"
  | "sword-slice"
  | "gunshot";

/** The looped layers of each Phase's background. */
const BACKGROUNDS: Record<Phase, AmbianceFile[]> = {
  night: ["night-music", "night-wind"],
  day: ["day-tavern"],
};

const SOUND_FILES: Record<Sound, AmbianceFile> = {
  howl: "wolf-howl",
  bottle: "bottle-smash",
  sword: "sword-slice",
  gunshot: "gunshot",
};

export const AMBIANCE_FILES: AmbianceFile[] = [...Object.values(BACKGROUNDS).flat(), ...Object.values(SOUND_FILES)];

/** Each bus's level until the Host sets their own: 1 is the file as it is. */
export const DEFAULT_LEVELS = { narrator: 1, ambiance: 0.6, effects: 0.9 };

/** The mix's tuning, as it sounded right at the table with the prototype. */
export const MIX = {
  layers: { "night-music": 0.7, "night-wind": 0.8, "day-tavern": 0.8 } as Partial<Record<AmbianceFile, number>>,
  /** The background's level, relative to its own, while the Narrator speaks. */
  duckLevel: 0.35,
  duckInSeconds: 0.3,
  duckOutSeconds: 0.8,
  /** The crossfade at nightfall and dawn. */
  fadeSeconds: 2.5,
};

/** What the mix needs of an AudioContext. */
export type MixContext = Pick<
  BaseAudioContext,
  "currentTime" | "destination" | "createGain" | "createBufferSource" | "createDynamicsCompressor"
>;

/**
 * Builds the mix on `context`, playing each file `buffer` has loaded; one not
 * loaded yet is skipped. `random` picks where a loop starts.
 */
export function createMixer(
  context: MixContext,
  buffer: (file: AmbianceFile) => AudioBuffer | undefined,
  random: () => number = Math.random,
) {
  const limiter = context.createDynamicsCompressor();
  limiter.threshold.value = -6;
  limiter.ratio.value = 12;
  limiter.connect(context.destination);
  const bus = (level: number, into: AudioNode) => {
    const gain = context.createGain();
    gain.gain.value = level;
    gain.connect(into);
    return gain;
  };
  const voice = bus(DEFAULT_LEVELS.narrator, limiter);
  const ducked = bus(1, limiter);
  const ambiance = bus(DEFAULT_LEVELS.ambiance, ducked);
  const effects = bus(DEFAULT_LEVELS.effects, limiter);

  const ramp = (param: AudioParam, to: number, seconds: number) => {
    const now = context.currentTime;
    param.cancelScheduledValues(now);
    param.setValueAtTime(param.value, now);
    param.linearRampToValueAtTime(to, now + seconds);
  };

  // The background layers playing, by file.
  const layers = new Map<AmbianceFile, { source: AudioBufferSourceNode; gain: GainNode }>();

  return {
    /** Where the Narrator's lines play into. */
    voice: voice as AudioNode,
    /**
     * Crossfades to `phase`'s background, or out to silence. The same Phase
     * again only starts the layers that loaded since.
     */
    setBackground(phase: Phase | null) {
      const wanted = new Set(phase ? BACKGROUNDS[phase] : []);
      for (const [file, layer] of layers) {
        if (wanted.has(file)) continue;
        ramp(layer.gain.gain, 0, MIX.fadeSeconds);
        layer.source.stop(context.currentTime + MIX.fadeSeconds + 0.1);
        layers.delete(file);
      }
      for (const file of wanted) {
        const loaded = buffer(file);
        if (layers.has(file) || !loaded) continue;
        const source = context.createBufferSource();
        source.buffer = loaded;
        source.loop = true;
        const gain = context.createGain();
        gain.gain.value = 0;
        source.connect(gain).connect(ambiance);
        // Somewhere inside the loop, so a long Game doesn't always hear the same opening.
        source.start(0, random() * loaded.duration);
        ramp(gain.gain, MIX.layers[file] ?? 1, MIX.fadeSeconds);
        layers.set(file, { source, gain });
      }
    },
    /** Plays a one-off sound, returning whether it could. */
    playSound(sound: Sound): boolean {
      const loaded = buffer(SOUND_FILES[sound]);
      if (!loaded) return false;
      const source = context.createBufferSource();
      source.buffer = loaded;
      source.connect(effects);
      source.start();
      return true;
    },
    /** The Narrator starts or stops speaking: the background dips under the voice. */
    duck(on: boolean) {
      ramp(ducked.gain, on ? MIX.duckLevel : 1, on ? MIX.duckInSeconds : MIX.duckOutSeconds);
    },
  };
}

export type Mixer = ReturnType<typeof createMixer>;
