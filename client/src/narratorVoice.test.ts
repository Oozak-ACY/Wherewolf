import { describe, expect, test } from "vitest";
import type { Cue } from "./narratorScript";
import { createNarratorVoice } from "./narratorVoice";

/**
 * Audio that records what it was asked to play, each line lasting until the
 * test ends it (or until it is stopped).
 */
function fakeAudio(mp3s: string[]) {
  const log: string[] = [];
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
      await new Promise((r) => setTimeout(r));
    },
    deps: {
      playMp3: (key: string) => (mp3s.includes(key) ? lasting(`mp3:${key}`) : null),
      speak: (text: string) => lasting(`speech:${text}`),
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
const deaths: Cue = { type: "live", text: "Cette nuit, le village a perdu P1." };

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
    await new Promise((r) => setTimeout(r));

    expect(audio.log).toEqual(["stop", "mp3:nightfall"]);
  });

  test("a fixed line whose MP3 is missing is spoken by the browser", async () => {
    const audio = fakeAudio(["seersTurn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([nightfall, seersTurn]);
    await audio.end();

    expect(audio.log).toEqual(["stop", "speech:La nuit tombe.", "mp3:seersTurn"]);
  });

  test("a live line is spoken by the browser", async () => {
    const audio = fakeAudio(["dawn"]);
    const voice = createNarratorVoice(audio.deps);

    voice.play([{ type: "fixed", key: "dawn", text: "Le jour se lève." }, deaths]);
    await audio.end();

    expect(audio.log).toEqual(["stop", "mp3:dawn", "speech:Cette nuit, le village a perdu P1."]);
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
    voice.play([{ type: "fixed", key: "dawn", text: "Le jour se lève." }]);
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
