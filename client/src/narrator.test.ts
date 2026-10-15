import { describe, expect, test } from "vitest";
import { chooseFrenchVoice, speaksAloud } from "./narrator";

describe("which phone reads the Narrator's lines aloud", () => {
  test("the Host's phone, the Table device, speaks", () => {
    expect(speaksAloud({ you: 1, host: 1 })).toBe(true);
  });

  test("every other phone stays silent", () => {
    expect(speaksAloud({ you: 2, host: 1 })).toBe(false);
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
