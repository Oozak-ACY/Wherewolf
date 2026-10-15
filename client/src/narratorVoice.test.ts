import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { Cue } from "./narratorScript";
import { createNarratorVoice, LINE_GAP_MS, LIVE_LINE_PATIENCE_MS, SOUND_LEAD_MS } from "./narratorVoice";

/**
 * Audio that records what it was asked to play, each line lasting until the
 * test ends it (or until it is stopped). Live lines load until the test
 * says whether they arrived.
 */
function fakeAudio(mp3s: string[], sounds = true) {
  const log: string[] = [];
  // Whether the background dips under the voice, each time it changes.
  const ducking: boolean[] = [];
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
    ducking,
    /** Ends the line playing, and lets the voice move on. */
    async end() {
      ending?.();
      ending = null;
      await vi.advanceTimersByTimeAsync(0);
    },
    /** Ends the line playing, and lets the silence before the next one pass. */
    async next() {
      await this.end();
      await vi.advanceTimersByTimeAsync(LINE_GAP_MS);
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
      playSound: (sound: string) => {
        if (sounds) log.push(`sound:${sound}`);
        return sounds;
      },
      setBackground: (phase: string | null) => log.push(`background:${phase}`),
      duck: (on: boolean) => ducking.push(on),
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
    await audio.next();

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
    await audio.next();

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
    await audio.next();

    expect(audio.log).toEqual(["stop", `load:${deaths.text}`, "mp3:dawn", `live:${deaths.text}`]);
  });

  test("a live line arriving a moment after its turn still plays", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.next();
    await vi.advanceTimersByTimeAsync(LIVE_LINE_PATIENCE_MS - 1);
    await audio.arrive(deaths.text);

    expect(audio.log.at(-1)).toBe(`live:${deaths.text}`);
  });

  test("a live line that failed is spoken by the browser", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.arrive(deaths.text, false);
    await audio.next();

    expect(audio.log.at(-1)).toBe(`speech:${deaths.text}`);
  });

  test("a live line still missing a few seconds after its turn is spoken by the browser, once", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([dawn, deaths]);
    await audio.next();
    await vi.advanceTimersByTimeAsync(LIVE_LINE_PATIENCE_MS);
    await audio.arrive(deaths.text);

    expect(audio.log).toEqual(["stop", `load:${deaths.text}`, "mp3:dawn", `speech:${deaths.text}`]);
  });

  test("a live line that arrived but cannot play is spoken by the browser", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice({ ...audio.deps, playLive: () => null });

    voice.play([dawn, deaths]);
    await audio.arrive(deaths.text);
    await audio.next();

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

  test("a new Moment's cues cut the previous ones", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn", "dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    voice.play([dawn]);
    await audio.end();
    await audio.end();

    expect(audio.log).toEqual(["stop", "mp3:nightfall", "stop", "mp3:dawn"]);
  });

  test("leaves a silence between two lines", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    await audio.end();
    await vi.advanceTimersByTimeAsync(LINE_GAP_MS - 1);
    expect(audio.log).toEqual(["stop", "mp3:nightfall"]);

    await vi.advanceTimersByTimeAsync(1);
    expect(audio.log).toEqual(["stop", "mp3:nightfall", "mp3:seersTurn"]);
  });

  test("says the Moment is announced as soon as its last line ends", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);
    const announced = vi.fn();

    voice.play([nightfall, seersTurn], announced);
    await audio.next();
    expect(announced).not.toHaveBeenCalled();

    await audio.end();
    expect(announced).toHaveBeenCalledOnce();
  });

  test("with no line to say, the Moment is announced at once", async () => {
    const audio = fakeAudio([]);
    const voice = createNarratorVoice(audio.deps);
    const announced = vi.fn();

    voice.play([{ type: "background", phase: "day" }], announced);
    await vi.advanceTimersByTimeAsync(0);

    expect(announced).toHaveBeenCalledOnce();
  });

  test("cues cut by a new Moment never say their Moment was announced", async () => {
    const audio = fakeAudio(["nightfall", "dawn"]);
    const voice = createNarratorVoice(audio.deps);
    const announced = vi.fn();

    voice.play([nightfall], announced);
    voice.play([dawn]);
    await audio.end();
    voice.stop();
    await audio.end();

    expect(announced).not.toHaveBeenCalled();
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

const night: Cue = { type: "background", phase: "night" };
const howl: Cue = { type: "sound", sound: "howl" };
const sword: Cue = { type: "sound", sound: "sword" };

describe("the ambiance under the Narrator", () => {
  test("a background changes at once, in its place among the cues", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, { type: "background", phase: "day" }, seersTurn]);
    expect(audio.log).toEqual(["stop", "mp3:nightfall"]);

    await audio.next();
    expect(audio.log).toEqual(["stop", "mp3:nightfall", "background:day", "mp3:seersTurn"]);
  });

  test("a background cut before its turn still changes, so the Phase never keeps the wrong one", async () => {
    const audio = fakeAudio(["witchSleeps", "dawn", "discussion"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([{ type: "fixed", key: "witchSleeps", text: "…" }, { type: "background", phase: "day" }, dawn]);
    voice.play([{ type: "fixed", key: "discussion", text: "Débattez." }]);

    expect(audio.log).toEqual(["stop", "mp3:witchSleeps", "background:day", "stop", "mp3:discussion"]);
  });

  test("a sound plays after the silence that follows a line, and its line starts over the sound's tail", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([night, nightfall, howl, seersTurn]);
    await audio.next();
    expect(audio.log).toEqual(["stop", "background:night", "mp3:nightfall", "sound:howl"]);

    await vi.advanceTimersByTimeAsync(SOUND_LEAD_MS.howl - 1);
    expect(audio.log.at(-1)).toBe("sound:howl");

    await vi.advanceTimersByTimeAsync(1);
    expect(audio.log.at(-1)).toBe("mp3:seersTurn");
  });

  test("a sound first in its cues plays at once", () => {
    const audio = fakeAudio(["seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([howl, seersTurn]);

    expect(audio.log).toEqual(["stop", "sound:howl"]);
  });

  test("a sound not loaded yet holds nothing up", () => {
    const audio = fakeAudio(["seersTurn"], false);
    const voice = createNarratorVoice(audio.deps);

    voice.play([howl, seersTurn]);

    expect(audio.log).toEqual(["stop", "mp3:seersTurn"]);
  });

  test("a new Moment's cues interrupt a sound's lead: the line after it is never said", async () => {
    const audio = fakeAudio(["seersTurn", "dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([howl, seersTurn]);
    voice.play([dawn]);
    await vi.advanceTimersByTimeAsync(SOUND_LEAD_MS.howl);

    expect(audio.log).toEqual(["stop", "sound:howl", "stop", "mp3:dawn"]);
  });

  test("a new Moment's cues drop the previous ones' sounds still to come", async () => {
    const audio = fakeAudio(["nightfall", "dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, sword, deaths]);
    voice.play([dawn]);
    await audio.next();
    await vi.advanceTimersByTimeAsync(LIVE_LINE_PATIENCE_MS);

    expect(audio.log).not.toContain("sound:sword");
  });

  test("the background dips from the first line to the last, silences between them included", async () => {
    const audio = fakeAudio(["nightfall", "seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([night, nightfall, seersTurn]);
    expect(audio.ducking).toEqual([true]);

    await audio.next();
    expect(audio.ducking).toEqual([true, true]);

    await audio.end();
    expect(audio.ducking).toEqual([true, true, false]);
  });

  test("the background stays up under a sound until its line starts", async () => {
    const audio = fakeAudio(["seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([howl, seersTurn]);
    expect(audio.ducking).toEqual([]);

    await vi.advanceTimersByTimeAsync(SOUND_LEAD_MS.howl);
    expect(audio.ducking).toEqual([true]);
  });

  test("the background comes back up when the Narrator is stopped", () => {
    const audio = fakeAudio(["nightfall"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall]);
    voice.stop();

    expect(audio.ducking.at(-1)).toBe(false);
  });

  test("cues with no line leave the background up", async () => {
    const audio = fakeAudio([]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([night]);
    await vi.advanceTimersByTimeAsync(0);

    expect(audio.ducking).toEqual([false]);
  });
});
