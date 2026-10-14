import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import {
  chooseFrenchVoice,
  createNarrator,
  narratorLine,
  speaksAloud,
  type NarratorSettings,
} from "./narrator";

/** A speech engine that only records what it was asked to say. */
function fakeSpeech() {
  const said: SpeechSynthesisUtterance[] = [];
  /** What the engine was told, in order: "cancel" or the text spoken. */
  const log: string[] = [];
  return {
    said,
    log,
    speak: (utterance: SpeechSynthesisUtterance) => {
      said.push(utterance);
      log.push(utterance.text);
    },
    cancel: () => log.push("cancel"),
  };
}

function setup(voice: SpeechSynthesisVoice | null = null) {
  const speech = fakeSpeech();
  const narrator = createNarrator({
    speech,
    utterance: (text) => ({ text }) as SpeechSynthesisUtterance,
    voice: () => voice,
  });
  return { speech, narrator };
}

describe("the Narrator speaking a line", () => {
  test("speaks in French with the chosen voice", () => {
    const thomas = { name: "Thomas", lang: "fr-FR" } as SpeechSynthesisVoice;
    const { speech, narrator } = setup(thomas);

    narrator.say("Le village vote.");

    expect(speech.said).toMatchObject([{ text: "Le village vote.", lang: "fr-FR", voice: thomas }]);
  });

  test("a new line cuts the previous one", () => {
    const { speech, narrator } = setup();

    narrator.say("Le village débat : qui sont les Loups-Garous ?");
    narrator.say("Le village vote.");

    expect(speech.log.slice(-3)).toEqual([
      "Le village débat : qui sont les Loups-Garous ?",
      "cancel",
      "Le village vote.",
    ]);
  });

  test("stopping silences the line playing", () => {
    const { speech, narrator } = setup();

    narrator.say("Le village vote.");
    narrator.stop();

    expect(speech.log.at(-1)).toBe("cancel");
  });
});

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

function player(id: number, alive = true, revealedRole: PlayerSummary["revealedRole"] = null) {
  return { id, name: `P${id}`, connected: true, alive, revealedRole } as PlayerSummary;
}

function settings(seer: number, hiddenRoles = false): NarratorSettings {
  return { roles: { werewolf: 1, seer, witch: 0, hunter: 0, villager: 4 - seer }, hiddenRoles };
}

describe("announcing the Role reveal", () => {
  test("asks everyone to read their Role, never naming one, once for the whole reveal", () => {
    const players = [1, 2, 3, 4, 5].map((id) => player(id));

    const first = narratorLine({ type: "roleReveal", stillReading: 5, confirmed: false }, players, settings(1));
    const later = narratorLine({ type: "roleReveal", stillReading: 1, confirmed: true }, players, settings(1));

    expect(first).toBe("Chacun découvre son rôle en secret sur son téléphone.");
    expect(later).toBe(first);
  });
});

describe("announcing the Night", () => {
  const werewolvesTurn = { type: "werewolvesTurn", picks: null } as const;

  test("night falls with the Seer's Turn, then the Werewolves wake", () => {
    const players = [1, 2, 3, 4, 5].map((id) => player(id));

    expect(narratorLine({ type: "seersTurn", inspection: null, passed: false }, players, settings(1))).toMatch(
      /^La nuit tombe\./,
    );
    expect(narratorLine(werewolvesTurn, players, settings(1))).not.toMatch(/La nuit tombe/);
  });

  test("without a Seer in play, night falls with the Werewolves' Turn", () => {
    const players = [1, 2, 3, 4, 5].map((id) => player(id));

    expect(narratorLine(werewolvesTurn, players, settings(0))).toMatch(/^La nuit tombe\./);
  });

  test("once the Seer is dead, night falls with the Werewolves' Turn", () => {
    const players = [player(1, false, "seer"), ...[2, 3, 4, 5].map((id) => player(id))];

    expect(narratorLine(werewolvesTurn, players, settings(1))).toMatch(/^La nuit tombe\./);
  });

  test("with Hidden Roles on, even a dead Seer's Turn opens the Night", () => {
    const players = [player(1, false, "seer"), ...[2, 3, 4, 5].map((id) => player(id))];

    expect(narratorLine(werewolvesTurn, players, settings(1, true))).not.toMatch(/La nuit tombe/);
  });

  test("the Witch wakes after the Werewolves, whatever she knows", () => {
    const players = [1, 2, 3, 4, 5].map((id) => player(id));
    const sight = { victim: 2, healed: false, poisoned: null, passed: false };

    for (const witch of [null, sight]) {
      const line = narratorLine({ type: "witchsTurn", witch }, players, settings(1));
      expect(line).toBe("La Sorcière se réveille.");
    }
  });
});

describe("announcing the dawn", () => {
  test("every death is announced without its cause", () => {
    const players = [player(1, false, "villager"), player(2, false, "seer"), player(3)];

    const line = narratorLine({ type: "dawn", deaths: [1, 2] }, players, settings(1));

    expect(line).toBe("Le jour se lève. Cette nuit, le village a perdu P1 (Villageois), P2 (Voyante).");
    expect(line).not.toMatch(/poison|loup|sorci/i);
  });

  test("with Hidden Roles on, the dead are announced without their Role", () => {
    const players = [player(1, false), player(2, false), player(3)];

    const line = narratorLine({ type: "dawn", deaths: [1, 2] }, players, settings(1, true));

    expect(line).toBe("Le jour se lève. Cette nuit, le village a perdu P1, P2.");
  });
});

describe("announcing the Vote's result", () => {
  test("the eliminated Player's Role is revealed, unless it is hidden", () => {
    const vote: Moment = { type: "voteResult", ballots: [], eliminated: 2 };

    expect(narratorLine(vote, [player(1), player(2, false, "witch")], settings(1))).toBe(
      "Le village a éliminé P2, qui était Sorcière.",
    );
    expect(narratorLine(vote, [player(1), player(2, false)], settings(1, true))).toBe(
      "Le village a éliminé P2.",
    );
  });
});

describe("announcing the Hunter's shot", () => {
  test("the eliminated Hunter is called to shoot", () => {
    const players = [player(1, false, "hunter"), player(2), player(3)];

    const line = narratorLine({ type: "huntersShot", hunter: 1 }, players, settings(1));

    expect(line).toBe("P1, le Chasseur, emporte quelqu'un avec lui : il choisit sur qui tirer.");
  });

  test("the shot reveals its target's Role", () => {
    const players = [player(1, false, "hunter"), player(2, false, "werewolf"), player(3)];

    const line = narratorLine({ type: "shotResult", hunter: 1, shot: 2 }, players, settings(1));

    expect(line).toBe("Le Chasseur a tiré sur P2, qui était Loup-Garou.");
  });

  test("with Hidden Roles on, the shot keeps its target's Role hidden", () => {
    const players = [player(1, false), player(2, false), player(3)];

    const line = narratorLine({ type: "shotResult", hunter: 1, shot: 2 }, players, settings(1, true));

    expect(line).toBe("Le Chasseur a tiré sur P2.");
  });

  test("a Hunter who did not shoot in time", () => {
    const players = [player(1, false, "hunter"), player(2), player(3)];

    const line = narratorLine({ type: "shotResult", hunter: 1, shot: null }, players, settings(1));

    expect(line).toBe("Le Chasseur n'a tiré sur personne.");
  });
});

describe("announcing the Mayor", () => {
  const players = [1, 2, 3, 4, 5].map((id) => player(id));
  const say = (moment: Moment) => narratorLine(moment, players, settings(1));

  test("the village elects its Mayor, and the result says whether it was drawn", () => {
    expect(say({ type: "election", voted: [], yourBallot: null })).toBe("Le village élit son Maire.");
    expect(say({ type: "electionResult", ballots: [], mayor: 2, byLot: false })).toBe(
      "P2 est élu Maire.",
    );
    expect(say({ type: "electionResult", ballots: [], mayor: 2, byLot: true })).toBe(
      "Le sort désigne P2 comme Maire.",
    );
  });

  test("a tied Vote goes to the Mayor", () => {
    const line = say({ type: "tieBreak", ballots: [], tied: [1, 2] });

    expect(line).toBe("Égalité entre P1, P2 : le Maire choisit qui est éliminé.");
  });

  test("the eliminated Mayor names a successor, or the lot does", () => {
    expect(say({ type: "succession", mayor: 1 })).toBe("P1, le Maire, désigne son successeur.");
    expect(say({ type: "successionResult", successor: 3, byLot: false })).toBe("P3 devient Maire.");
    expect(say({ type: "successionResult", successor: 3, byLot: true })).toBe(
      "Le sort désigne P3 comme nouveau Maire.",
    );
  });
});
