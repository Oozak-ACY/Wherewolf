// PROTOTYPE, throwaway. The Table device's sound mix, with Web Audio:
//   bed   — looped backgrounds for the Phase (Night: music + wind, Day: tavern), crossfaded
//   sfx   — one-off sounds tied to a Moment (howl, slash, gunshot…)
//   voice — the Narrator's <audio> element, routed through Web Audio so its
//           volume can be set (iOS ignores `audio.volume`)
//   duck  — the bed drops while the Narrator speaks
// Everything ends in a compressor, so a boosted voice or effect doesn't clip.
// Files live in public/audio (Pixabay downloads, renamed).

export const SOUNDS = {
  "night-music": "Night · suspense music (loop)",
  "night-wind": "Night · wind & whispers (loop)",
  "day-tavern": "Day · tavern ambience (loop)",
  "wolf-howl": "Werewolves' Turn · howl",
  "sword-slice": "Vote · execution slash",
  gunshot: "Hunter's shot · gunshot",
  "bottle-smash": "Witch's Turn · bottle smash",
  kiss: "Cupid's Turn (future Role) · kiss",
};

export const BEDS = {
  night: ["night-music", "night-wind"],
  day: ["day-tavern"],
  none: [],
};

let ctx;
let master, bedBus, duckGain, sfxBus, voiceBus;
const buffers = {};
const playingBeds = new Map(); // sound name -> { src, gain }
// The Host's mixing table (voice, bed, sfx: 0 to 1.5) and the prototype's tuning (the rest).
export const mix = { voice: 1, bed: 0.6, sfx: 0.9, layer: { "night-music": 0.7, "night-wind": 0.8, "day-tavern": 0.8 }, duck: 0.35, fade: 2.5 };
export let currentBed = "none";

/** Must run inside a tap: creates and resumes the AudioContext, then loads every file. */
export async function unlockAmbiance(voiceElement, onLoaded) {
  // iOS: without this, the silent switch mutes Web Audio.
  try { if (navigator.audioSession) navigator.audioSession.type = "playback"; } catch {}
  ctx ??= new AudioContext();
  await ctx.resume();
  const limiter = ctx.createDynamicsCompressor();
  limiter.threshold.value = -6;
  limiter.ratio.value = 12;
  limiter.connect(ctx.destination);
  master = ctx.createGain();
  master.connect(limiter);
  voiceBus = ctx.createGain();
  voiceBus.gain.value = mix.voice;
  voiceBus.connect(master);
  ctx.createMediaElementSource(voiceElement).connect(voiceBus);
  duckGain = ctx.createGain();
  duckGain.connect(master);
  bedBus = ctx.createGain();
  bedBus.gain.value = mix.bed;
  bedBus.connect(duckGain);
  sfxBus = ctx.createGain();
  sfxBus.gain.value = mix.sfx;
  sfxBus.connect(master);
  await Promise.all(
    Object.keys(SOUNDS).map(async (name) => {
      const data = await (await fetch(`/audio/${name}.mp3`)).arrayBuffer();
      buffers[name] = await ctx.decodeAudioData(data);
      onLoaded?.(name, buffers[name].duration);
    }),
  );
}

const ramp = (param, to, seconds) => {
  const now = ctx.currentTime;
  param.cancelScheduledValues(now);
  param.setValueAtTime(param.value, now);
  param.linearRampToValueAtTime(to, now + seconds);
};

/** Crossfades to the Phase's background layers. */
export function setBed(bed) {
  if (!ctx || bed === currentBed) return;
  currentBed = bed;
  const wanted = new Set(BEDS[bed]);
  for (const [name, layer] of playingBeds) {
    if (wanted.has(name)) continue;
    ramp(layer.gain.gain, 0, mix.fade);
    layer.src.stop(ctx.currentTime + mix.fade + 0.1);
    playingBeds.delete(name);
  }
  for (const name of wanted) {
    if (playingBeds.has(name)) continue;
    const src = ctx.createBufferSource();
    src.buffer = buffers[name];
    src.loop = true;
    const gain = ctx.createGain();
    gain.gain.value = 0;
    src.connect(gain).connect(bedBus);
    // Start somewhere random so a long Game doesn't always hear the same opening.
    src.start(0, Math.random() * src.buffer.duration);
    ramp(gain.gain, mix.layer[name] ?? 1, mix.fade);
    playingBeds.set(name, { src, gain });
  }
}

/** Plays a one-off sound; resolves after `leadMs` so the Narrator can start over its tail. */
export function cue(name, leadMs = 0) {
  if (!ctx || !buffers[name]) return Promise.resolve();
  const src = ctx.createBufferSource();
  src.buffer = buffers[name];
  src.connect(sfxBus);
  src.start();
  return new Promise((r) => setTimeout(r, leadMs));
}

/** The Narrator starts or stops speaking: the bed dips under the voice. */
export function duck(on) {
  if (!ctx) return;
  ramp(duckGain.gain, on ? mix.duck : 1, on ? 0.3 : 0.8);
}

export function applyMix() {
  if (!ctx) return;
  ramp(voiceBus.gain, mix.voice, 0.1);
  ramp(bedBus.gain, mix.bed, 0.1);
  ramp(sfxBus.gain, mix.sfx, 0.1);
  for (const [name, layer] of playingBeds) ramp(layer.gain.gain, mix.layer[name] ?? 1, 0.1);
}

export function stopAll() {
  setBed("none");
}

export const isReady = () => !!ctx && Object.keys(buffers).length === Object.keys(SOUNDS).length;
