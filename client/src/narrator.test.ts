import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { RoleCounts } from "./generated/RoleCounts";
import { chooseFrenchVoice, createNarrator, lineTimeoutMs, narratorLine } from "./narrator";

/** A speech engine that only records what it was asked to say. */
function fakeSpeech() {
  const said: SpeechSynthesisUtterance[] = [];
  return {
    said,
    speak: (utterance: SpeechSynthesisUtterance) => said.push(utterance),
    cancel: vi.fn(),
    /** The browser finished the latest line. */
    finish: () => said.at(-1)?.onend?.({} as SpeechSynthesisEvent),
  };
}

function fakeMic() {
  return { muted: false, mute() { this.muted = true; }, unmute() { this.muted = false; } };
}

function setup(voice: SpeechSynthesisVoice | null = null) {
  const speech = fakeSpeech();
  const mic = fakeMic();
  const narrator = createNarrator({
    speech,
    utterance: (text) => ({ text }) as SpeechSynthesisUtterance,
    mic,
    voice: () => voice,
  });
  return { speech, mic, narrator };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("the Narrator speaking a line", () => {
  test("re-enables the mic after the timeout when onend never fires", () => {
    const { mic, narrator } = setup();
    const line = "Le village vote.";

    narrator.say(line);
    expect(mic.muted).toBe(true);

    vi.advanceTimersByTime(lineTimeoutMs(line) - 1);
    expect(mic.muted).toBe(true);
    vi.advanceTimersByTime(1);
    expect(mic.muted).toBe(false);
  });

  test("re-enables the mic as soon as the line ends", () => {
    const { speech, mic, narrator } = setup();

    narrator.say("Le village vote.");
    speech.finish();

    expect(mic.muted).toBe(false);
  });

  test("a new line cuts the previous one and keeps the mic muted until it ends", () => {
    const { speech, mic, narrator } = setup();
    const first = "Le village débat : qui sont les Loups-Garous ?";
    const second = "Le village vote.";

    narrator.say(first);
    vi.advanceTimersByTime(lineTimeoutMs(first) - 100);
    narrator.say(second);
    expect(speech.cancel).toHaveBeenCalled();

    // The first line's timeout and a late onend must not unmute during the second.
    vi.advanceTimersByTime(200);
    speech.said[0].onend?.({} as SpeechSynthesisEvent);
    expect(mic.muted).toBe(true);

    vi.advanceTimersByTime(lineTimeoutMs(second));
    expect(mic.muted).toBe(false);
  });

  test("stopping mid-line silences it and gives the mic back", () => {
    const { speech, mic, narrator } = setup();

    narrator.say("Le village vote.");
    narrator.stop();

    expect(speech.cancel).toHaveBeenCalled();
    expect(mic.muted).toBe(false);
  });

  test("speaks in French with the chosen voice", () => {
    const thomas = { name: "Thomas", lang: "fr-FR" } as SpeechSynthesisVoice;
    const { speech, narrator } = setup(thomas);

    narrator.say("Le village vote.");

    expect(speech.said).toMatchObject([{ text: "Le village vote.", lang: "fr-FR", voice: thomas }]);
  });
});

function voice(name: string, lang: string, voiceURI = name) {
  return { name, lang, voiceURI } as SpeechSynthesisVoice;
}

describe("choosing the Narrator's voice", () => {
  test("never picks a novelty voice, even when the browser lists it first", () => {
    // What iOS Safari lists: its first fr-FR voice is "Grandma".
    const grandma = voice("Grandma", "fr-FR", "com.apple.eloquence.fr-FR.Grandma");
    const thomas = voice("Thomas", "fr-FR", "com.apple.voice.compact.fr-FR.Thomas");

    expect(chooseFrenchVoice([grandma, thomas])).toBe(thomas);
  });

  test("keeps real voices that share a URI scheme with the novelty ones", () => {
    // Older macOS Safari lists its ordinary voices under this scheme too.
    const thomas = voice("Thomas", "fr-FR", "com.apple.speech.synthesis.voice.thomas");

    expect(chooseFrenchVoice([thomas])).toBe(thomas);
  });

  test("prefers a known good fr-FR voice over other French voices", () => {
    const canadian = voice("Amélie", "fr-CA");
    const unknown = voice("Jacques", "fr-FR");
    const google = voice("Google français", "fr-FR");

    expect(chooseFrenchVoice([canadian, unknown, google])).toBe(google);
  });

  test("falls back to any fr-FR voice, then any French one", () => {
    const canadian = voice("Amélie", "fr-CA");
    const unknown = voice("Jacques", "fr_FR");

    expect(chooseFrenchVoice([canadian, unknown])).toBe(unknown);
    expect(chooseFrenchVoice([canadian])).toBe(canadian);
    expect(chooseFrenchVoice([voice("Samantha", "en-US")])).toBeNull();
  });
});

function player(id: number, alive = true, revealedRole: PlayerSummary["revealedRole"] = null) {
  return { id, name: `P${id}`, connected: true, alive, revealedRole } as PlayerSummary;
}

function roles(seer: number): RoleCounts {
  return { werewolf: 1, seer, witch: 0, hunter: 0, villager: 4 - seer };
}

describe("announcing the Night", () => {
  const werewolvesTurn = { type: "werewolvesTurn", picks: null } as const;

  test("night falls with the Seer's Turn, then the Werewolves wake", () => {
    const players = [1, 2, 3, 4, 5].map((id) => player(id));

    expect(narratorLine({ type: "seersTurn", inspection: null }, players, roles(1))).toMatch(
      /^La nuit tombe\./,
    );
    expect(narratorLine(werewolvesTurn, players, roles(1))).not.toMatch(/La nuit tombe/);
  });

  test("without a Seer in play, night falls with the Werewolves' Turn", () => {
    const players = [1, 2, 3, 4, 5].map((id) => player(id));

    expect(narratorLine(werewolvesTurn, players, roles(0))).toMatch(/^La nuit tombe\./);
  });

  test("once the Seer is dead, night falls with the Werewolves' Turn", () => {
    const players = [player(1, false, "seer"), ...[2, 3, 4, 5].map((id) => player(id))];

    expect(narratorLine(werewolvesTurn, players, roles(1))).toMatch(/^La nuit tombe\./);
  });
});
