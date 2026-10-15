// The browser's speech synthesis, the Narrator's fallback voice, and which
// phone speaks.

import type { PlayerView } from "./generated/PlayerView";

/**
 * Must run synchronously inside a tap. iOS only lets a page speak
 * later, without a tap, if it already spoke once during a user gesture
 * (validated on real phones, see issue #1).
 */
export function unlockSpeech(): void {
  if (!("speechSynthesis" in window)) return;
  window.speechSynthesis.speak(new SpeechSynthesisUtterance(""));
}

// Apple's joke voices, listed among the real ones in every language.
const NOVELTY_VOICES = [
  "Albert", "Bad News", "Bahh", "Bells", "Boing", "Bubbles", "Cellos", "Eddy",
  "Flo", "Fred", "Good News", "Grandma", "Grandpa", "Jester", "Junior", "Kathy",
  "Organ", "Ralph", "Reed", "Rocko", "Sandy", "Shelley", "Superstar", "Trinoids",
  "Whisper", "Wobble", "Zarvox",
];

// Natural-sounding fr-FR voices across iOS, Android, Chrome, Edge and Windows,
// best first. Matched as part of the name ("Microsoft Denise Online (Natural)…").
const PREFERRED_VOICES = [
  "Microsoft Denise", "Microsoft Henri", "Microsoft Vivienne", "Microsoft Remy",
  "Google français", "Audrey", "Aurélie", "Thomas", "Marie", "Hortense", "Julie", "Paul",
];

function isNovelty(voice: SpeechSynthesisVoice): boolean {
  return (
    voice.voiceURI.includes("eloquence") ||
    NOVELTY_VOICES.some((name) => voice.name === name || voice.name.startsWith(`${name} `))
  );
}

/** Lower is better: fr-FR before other French, then the preferred list, then the rest. */
function rank(voice: SpeechSynthesisVoice): [number, number] {
  const france = voice.lang.replace("_", "-").toLowerCase() === "fr-fr" ? 0 : 1;
  const preferred = PREFERRED_VOICES.findIndex((name) => voice.name.includes(name));
  return [france, preferred === -1 ? PREFERRED_VOICES.length : preferred];
}

/**
 * The Narrator's voice among the browser's. iOS would default to its first
 * `fr-FR` voice, which can be a novelty one like "Grandma" (issue #1).
 */
export function chooseFrenchVoice(voices: SpeechSynthesisVoice[]): SpeechSynthesisVoice | null {
  const french = voices.filter((v) => v.lang.toLowerCase().startsWith("fr") && !isNovelty(v));
  const ranked = french.map((v) => ({ v, rank: rank(v) }));
  ranked.sort((a, b) => a.rank[0] - b.rank[0] || a.rank[1] - b.rank[1]);
  return ranked[0]?.v ?? null;
}

/**
 * Whether this phone reads the Narrator's lines aloud: only the Table device,
 * the Host's phone, does. Every other phone shows them silently.
 */
export function speaksAloud({ you, host }: Pick<PlayerView, "you" | "host">): boolean {
  return you === host;
}
