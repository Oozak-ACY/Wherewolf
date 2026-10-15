import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { Cue } from "./narratorScript";
import { createNarratorVoice, LIVE_LINE_PATIENCE_MS } from "./narratorVoice";

/**
 * Audio that records what it was asked to play, each line lasting until the
 * test ends it (or until it is stopped). Live lines load until the test
 * says whether they arrived.
 */
function fakeAudio(mp3s: string[]) {
  const log: string[] = [];
  const loading = new Map<string, (arrived: boolean) => void>();
  const arrived = new Set<string>();
  let ending: (() => void) | null = null;
  const lasting = (entry: string) => {
    log.push(entry);
    return new Promise<void>((resolve) => {
      ending = resolve;
    });
  };
  return {
    log,
    /** Ends the line playing, and lets the voice move on. */
    async end() {
      ending?.();
      ending = null;
      await vi.advanceTimersByTimeAsync(0);
    },
    /** A live line's audio arrives from the server, or fails to. */
    async arrive(text: string, ok = true) {
      if (ok) arrived.add(text);
      loading.get(text)?.(ok);
      await vi.advanceTimersByTimeAsync(0);
    },
    deps: {
      playMp3: (key: string) => (mp3s.includes(key) ? lasting(`mp3:${key}`) : null),
      speak: (text: string) => lasting(`speech:${text}`),
      loadLive: (text: string) => {
        log.push(`load:${text}`);
        return new Promise<boolean>((resolve) => loading.set(text, resolve));
      },
      playLive: (text: string) => (arrived.has(text) ? lasting(`live:${text}`) : null),
      stop: () => {
        log.push("stop");
        ending?.();
        ending = null;
      },
    },
  };
}

const nightfall: Cue = { type: "fixed", key: "nightfall", text: "La nuit tombe." };
const seersTurn: Cue = { type: "fixed", key: "seersTurn", text: "La Voyante se réveille." };
const dawn: Cue = { type: "fixed", key: "dawn", text: "Le jour se lève." };
const deaths: Cue = { type: "live", text: "Cette nuit, le village a perdu P1." };

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("the Narrator's voice on the Table device", () => {
  test("plays fixed lines from their MP3, one after the other", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    await audio.end();

    expect(audio.log).toEqual(["stop", "mp3:nightfall", "mp3:seersTurn"]);
  });

  test("waits for a line to end before the next one", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    await vi.advanceTimersByTimeAsync(0);

    expect(audio.log).toEqual(["stop", "mp3:nightfall"]);
  });

  test("a fixed line whose MP3 is missing is spoken by the browser", async () => {
    const audio = fakeAudio(["seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    await audio.end();

    expect(audio.log).toEqual(["stop", "speech:La nuit tombe.", "mp3:seersTurn"]);
  });

  test("a live line is requested as soon as the cues arrive, before the lines ahead of it play", () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);

    expect(audio.log).toEqual(["stop", `load:${deaths.text}`, "mp3:dawn"]);
  });

  test("a live line plays right after its fixed part", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.arrive(deaths.text);
    await audio.end();

    expect(audio.log).toEqual(["stop", `load:${deaths.text}`, "mp3:dawn", `live:${deaths.text}`]);
  });

  test("a live line arriving a moment after its turn still plays", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.end();
    await vi.advanceTimersByTimeAsync(LIVE_LINE_PATIENCE_MS - 1);
    await audio.arrive(deaths.text);

    expect(audio.log.at(-1)).toBe(`live:${deaths.text}`);
  });

  test("a live line that failed is spoken by the browser", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.arrive(deaths.text, false);
    await audio.end();

    expect(audio.log.at(-1)).toBe(`speech:${deaths.text}`);
  });

  test("a live line still missing a few seconds after its turn is spoken by the browser, once", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.end();
    await vi.advanceTimersByTimeAsync(LIVE_LINE_PATIENCE_MS);
    await audio.arrive(deaths.text);

    expect(audio.log).toEqual(["stop", `load:${deaths.text}`, "mp3:dawn", `speech:${deaths.text}`]);
  });

  test("a live line that arrived but cannot play is spoken by the browser", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice({ ...audio.deps, playLive: () => null });

    voice.play([dawn, deaths]);
    await audio.arrive(deaths.text);
    await audio.end();

    expect(audio.log.at(-1)).toBe(`speech:${deaths.text}`);
  });

  test("a live line still loading when a new Moment comes is never said", async () => {
    const audio = fakeAudio(["dawn", "discussion"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.end();
    voice.play([{ type: "fixed", key: "discussion", text: "Débattez." }]);
    await audio.arrive(deaths.text);
    await vi.advanceTimersByTimeAsync(LIVE_LINE_PATIENCE_MS);

    expect(audio.log).toEqual(["stop", `load:${deaths.text}`, "mp3:dawn", "stop", "mp3:discussion"]);
  });

  test("sounds and backgrounds never hold up the voice", async () => {
    const audio = fakeAudio(["nightfall"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([{ type: "background", phase: "night" }, nightfall, { type: "sound", sound: "howl" }, seersTurn]);
    await audio.end();

    expect(audio.log).toEqual(["stop", "mp3:nightfall", "speech:La Voyante se réveille."]);
  });

  test("a new Moment's cues cut the previous ones", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn", "dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    voice.play([dawn]);
    await audio.end();
    await audio.end();

    expect(audio.log).toEqual(["stop", "mp3:nightfall", "stop", "mp3:dawn"]);
  });

  test("stopping silences the line playing and drops the rest", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    voice.stop();
    await audio.end();

    expect(audio.log).toEqual(["stop", "mp3:nightfall", "stop"]);
  });
});
