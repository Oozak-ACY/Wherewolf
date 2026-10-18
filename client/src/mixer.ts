// The Table device's sound mix, with Web Audio. Three buses, Narrator, Music
// & ambiance and Sound effects, each at the Host's level, end in one limiter,
// so a boosted level never clips. The ambiance bus dips while the Narrator
// speaks.

import type { Phase, Sound } from "./narratorScript";

/** A sound file of the ambiance, in `public/ambiance` (see its `CREDITS.md`). */
export type AmbianceFile =
  | "night-music"
  | "night-wind"
  | "day-tavern"
  | "wolf-howl"
  | "bottle-smash"
  | "sword-slice"
  | "gunshot"
  | "rooster";

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
  rooster: "rooster",
};

export const AMBIANCE_FILES: AmbianceFile[] = [...Object.values(BACKGROUNDS).flat(), ...Object.values(SOUND_FILES)];

/** Each bus's level, as the Host's faders set it: 1 is the file as it is. */
export type MixerLevels = { narrator: number; ambiance: number; effects: number };

/** Each bus's level until the Host sets their own. */
export const DEFAULT_LEVELS: MixerLevels = { narrator: 1, ambiance: 0.6, effects: 0.9 };

/** The buses, in the order of the faders and of the test sound's notes. */
export const BUSES: (keyof MixerLevels)[] = ["narrator", "ambiance", "effects"];

/** The faders' boost: 150%. */
export const MAX_LEVEL = 1.5;

/** `levels` made safe: each between silence and the boost, its default if it isn't a number. */
export function clampLevels(levels: Partial<Record<keyof MixerLevels, unknown>>): MixerLevels {
  const clamp = (bus: keyof MixerLevels) => {
    const level = levels[bus];
    if (typeof level !== "number" || Number.isNaN(level)) return DEFAULT_LEVELS[bus];
    return Math.min(MAX_LEVEL, Math.max(0, level));
  };
  return Object.fromEntries(BUSES.map((bus) => [bus, clamp(bus)])) as MixerLevels;
}

/** The mix's tuning, as it sounded right at the table with the prototype. */
export const MIX = {
  /** The tavern sits 30% under the prototype's tuning, so the Players can argue over its chatter. */
  layers: { "night-music": 0.7, "night-wind": 0.8, "day-tavern": 0.56 } as Partial<Record<AmbianceFile, number>>,
  /** The background's level, relative to its own, while the Narrator speaks. */
  duckLevel: 0.35,
  duckInSeconds: 0.3,
  duckOutSeconds: 0.8,
  /** The crossfade at nightfall and dawn. */
  fadeSeconds: 2.5,
  /** A fader's glide to its new level, short enough to feel instant, without a click. */
  levelSeconds: 0.05,
  /** The test sound: one note per bus, in the order of `BUSES`. */
  testNotes: [523.25, 659.25, 783.99],
  testNoteSeconds: 0.35,
  testNoteLevel: 0.4,
};

/** What the mix needs of an AudioContext. */
export type MixContext = Pick<
  BaseAudioContext,
  "currentTime" | "destination" | "createGain" | "createBufferSource" | "createDynamicsCompressor" | "createOscillator"
>;

/**
 * Builds the mix on `context` at the Host's `levels`, playing each file
 * `buffer` has loaded; one not loaded yet is skipped. `random` picks where a
 * loop starts.
 */
export function createMixer(
  context: MixContext,
  buffer: (file: AmbianceFile) => AudioBuffer | undefined,
  random: () => number = Math.random,
  levels: MixerLevels = DEFAULT_LEVELS,
) {
  const start = clampLevels(levels);
  const limiter = context.createDynamicsCompressor();
  // As hard as a compressor gets: three buses boosted to 150% still never clip.
  limiter.threshold.value = -6;
  limiter.knee.value = 0;
  limiter.ratio.value = 20;
  limiter.attack.value = 0.001;
  limiter.connect(context.destination);
  const bus = (level: number, into: AudioNode) => {
    const gain = context.createGain();
    gain.gain.value = level;
    gain.connect(into);
    return gain;
  };
  const ducked = bus(1, limiter);
  const buses: Record<keyof MixerLevels, GainNode> = {
    narrator: bus(start.narrator, limiter),
    ambiance: bus(start.ambiance, ducked),
    effects: bus(start.effects, limiter),
  };

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
    voice: buses.narrator as AudioNode,
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
        source.connect(gain).connect(buses.ambiance);
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
      source.connect(buses.effects);
      source.start();
      return true;
    },
    /** The Host moves a fader: each bus glides to its level. */
    setLevels(levels: MixerLevels) {
      const safe = clampLevels(levels);
      for (const name of BUSES) ramp(buses[name].gain, safe[name], MIX.levelSeconds);
    },
    /** A short chime, one note through each bus in turn, so the Host hears each fader. */
    playTest() {
      BUSES.forEach((name, index) => {
        const at = context.currentTime + index * MIX.testNoteSeconds;
        const note = context.createOscillator();
        note.frequency.value = MIX.testNotes[index]!;
        const envelope = context.createGain();
        envelope.gain.value = 0;
        envelope.gain.setValueAtTime(0, at);
        envelope.gain.linearRampToValueAtTime(MIX.testNoteLevel, at + 0.02);
        envelope.gain.linearRampToValueAtTime(0, at + MIX.testNoteSeconds);
        note.connect(envelope).connect(buses[name]);
        note.start(at);
        note.stop(at + MIX.testNoteSeconds + 0.05);
      });
    },
    /** The Narrator starts or stops speaking: the background dips under the voice. */
    duck(on: boolean) {
      ramp(ducked.gain, on ? MIX.duckLevel : 1, on ? MIX.duckInSeconds : MIX.duckOutSeconds);
    },
  };
}

export type Mixer = ReturnType<typeof createMixer>;
