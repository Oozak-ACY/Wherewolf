// What the Narrator says, and its voice: the browser's speech synthesis.

import type { Moment } from "./generated/Moment";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { RoleCounts } from "./generated/RoleCounts";
import { fr } from "./strings";

/** What the Narrator announces at `moment`: shown in the Game panel and read aloud. */
export function narratorLine(moment: Moment, players: PlayerSummary[], roles: RoleCounts): string {
  const name = (id: PlayerId) => players.find((p) => p.id === id)?.name ?? "?";
  const roleOf = (id: PlayerId) => {
    const role = players.find((p) => p.id === id)?.revealedRole;
    return role ? fr.roles[role].name : "?";
  };
  switch (moment.type) {
    case "seersTurn":
      return `${fr.narrator.nightfall} ${fr.narrator.seersTurn}`;
    case "werewolvesTurn":
      return seerWakes(players, roles)
        ? fr.narrator.werewolvesTurn
        : `${fr.narrator.nightfall} ${fr.narrator.werewolvesTurn}`;
    case "witchsTurn":
      return fr.narrator.witchsTurn;
    case "dawn":
      return moment.deaths.length === 0
        ? fr.narrator.dawnNobody
        : fr.narrator.dawnDeaths(
            moment.deaths.map((id) => `${name(id)} (${roleOf(id)})`).join(", "),
          );
    case "discussion":
      return fr.narrator.discussion;
    case "election":
      return fr.narrator.election;
    case "electionResult":
      return moment.byLot ? fr.narrator.electedByLot(name(moment.mayor)) : fr.narrator.elected(name(moment.mayor));
    case "tieBreak":
      return fr.narrator.tieBreak(moment.tied.map(name).join(", "));
    case "vote":
      return fr.narrator.vote;
    case "voteResult":
      return moment.eliminated === null
        ? fr.narrator.voteNobody
        : fr.narrator.voteEliminated(name(moment.eliminated), roleOf(moment.eliminated));
    case "huntersShot":
      return fr.narrator.huntersShot(name(moment.hunter));
    case "shotResult":
      return moment.shot === null
        ? fr.narrator.shotLost
        : fr.narrator.shot(name(moment.shot), roleOf(moment.shot));
    case "succession":
      return fr.narrator.succession(name(moment.mayor));
    case "successionResult":
      return moment.byLot
        ? fr.narrator.successorByLot(name(moment.successor))
        : fr.narrator.successor(name(moment.successor));
    case "victory":
      return fr.narrator.victory[moment.winner];
  }
}

/**
 * Whether the Night opens with the Seer's Turn: a Seer is in play and nobody
 * has seen her die. Otherwise the Werewolves' Turn opens it.
 */
function seerWakes(players: PlayerSummary[], roles: RoleCounts): boolean {
  return roles.seer > 0 && !players.some((p) => !p.alive && p.revealedRole === "seer");
}

/**
 * Must run synchronously inside the "Rejoindre" tap. iOS only lets a page speak
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

/** This Player's microphone, as the Narrator needs it. */
export type Mic = { mute(): void; unmute(): void };

export type NarratorDeps = {
  speech: Pick<SpeechSynthesis, "speak" | "cancel">;
  utterance: (text: string) => SpeechSynthesisUtterance;
  mic: Mic;
  /** The voice to speak with, looked up at each line: voices load late. */
  voice: () => SpeechSynthesisVoice | null;
};

/**
 * How long a line may keep the mic muted. `onend` never fires when speech is
 * blocked, so the mic comes back after this even if the browser stays silent.
 */
export function lineTimeoutMs(text: string): number {
  return 1000 + 80 * text.length;
}

/** Reads the Narrator's lines aloud, muting this Player's mic while each one plays. */
export function createNarrator({ speech, utterance, mic, voice }: NarratorDeps) {
  // The line playing now; an earlier line's end no longer concerns the mic.
  let playing: SpeechSynthesisUtterance | null = null;
  let timeout: ReturnType<typeof setTimeout> | undefined;

  const done = (line: SpeechSynthesisUtterance) => {
    if (line !== playing) return;
    playing = null;
    clearTimeout(timeout);
    mic.unmute();
  };

  return {
    say(text: string) {
      clearTimeout(timeout);
      if (playing) {
        playing = null;
        speech.cancel();
      }
      const line = utterance(text);
      line.lang = "fr-FR";
      line.voice = voice();
      playing = line;
      mic.mute();
      timeout = setTimeout(() => done(line), lineTimeoutMs(text));
      line.onend = () => done(line);
      line.onerror = () => done(line);
      speech.speak(line);
    },
    /** Silences the current line, if any, and gives the mic back. */
    stop() {
      const line = playing;
      if (!line) return;
      speech.cancel();
      done(line);
    },
  };
}
