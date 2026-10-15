import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import { narratorCues, narratorText, type NarratorView } from "./narratorScript";

function player(id: number, alive = true, revealedRole: PlayerSummary["revealedRole"] = null) {
  return { id, name: `P${id}`, connected: true, alive, revealedRole } as PlayerSummary;
}

const everyone = [1, 2, 3, 4, 5].map((id) => player(id));

function view(moment: Moment | null, players = everyone, hiddenRoles = false, paused = false): NarratorView {
  return { moment, players, paused, settings: { hiddenRoles } };
}

const roleReveal: Moment = { type: "roleReveal", stillReading: 5, confirmed: false };

describe("the Role reveal", () => {
  test("asks everyone to read their Role, never naming one", () => {
    expect(narratorCues(view(null), view(roleReveal))).toEqual([
      { type: "fixed", key: "roleReveal", text: "Chacun découvre son rôle en secret sur son téléphone." },
    ]);
  });

  test("says it once for the whole reveal", () => {
    const later: Moment = { type: "roleReveal", stillReading: 1, confirmed: true };

    expect(narratorCues(view(roleReveal), view(later))).toEqual([]);
  });

  test("the Lobby says nothing", () => {
    expect(narratorCues(null, view(null))).toEqual([]);
    expect(narratorText(view(null))).toBeNull();
  });
});

const seersTurn: Moment = { type: "seersTurn", inspection: null, passed: false };
const werewolvesTurn: Moment = { type: "werewolvesTurn", picks: null };
const witchsTurn: Moment = { type: "witchsTurn", witch: null };
const vote: Moment = { type: "vote", voted: [], yourBallot: null };

/** What the table hears: fixed lines by key, live lines by text, sounds and backgrounds. */
function heard(cues: ReturnType<typeof narratorCues>) {
  return cues.map((cue) => {
    switch (cue.type) {
      case "fixed":
        return cue.key;
      case "live":
        return cue.text;
      case "sound":
        return `sound:${cue.sound}`;
      case "background":
        return `background:${cue.phase}`;
    }
  });
}

describe("the Night", () => {
  test("night falls once every Role is seen, with the Night background, then the Seer wakes", () => {
    expect(heard(narratorCues(view(roleReveal), view(seersTurn)))).toEqual([
      "background:night",
      "nightfall",
      "seersTurn",
    ]);
  });

  test("the Seer goes back to sleep before the Werewolves wake, to a howl", () => {
    expect(heard(narratorCues(view(seersTurn), view(werewolvesTurn)))).toEqual([
      "seerSleeps",
      "sound:howl",
      "werewolvesTurn",
    ]);
  });

  test("the Werewolves go back to sleep before the Witch wakes, to a bottle smash", () => {
    expect(heard(narratorCues(view(werewolvesTurn), view(witchsTurn)))).toEqual([
      "werewolvesSleep",
      "sound:bottle",
      "witchsTurn",
    ]);
  });

  test("without the Seer's Turn, night falls with the Werewolves'", () => {
    expect(heard(narratorCues(view(vote), view(werewolvesTurn)))).toEqual([
      "background:night",
      "nightfall",
      "sound:howl",
      "werewolvesTurn",
    ]);
  });

  test("the Seer's reading Moment says nothing new", () => {
    const reading: Moment = { type: "seersTurn", inspection: { player: 2, role: "werewolf" }, passed: false };

    expect(narratorCues(view(seersTurn), view(reading))).toEqual([]);
  });

  test("nothing a Role does during its Turn makes a sound", () => {
    const before: Moment = { type: "witchsTurn", witch: { victim: 2, healed: false, poisoned: null, passed: false } };
    const healed: Moment = { type: "witchsTurn", witch: { victim: 2, healed: true, poisoned: 3, passed: false } };
    const picked: Moment = { type: "werewolvesTurn", picks: [{ werewolf: 1, victim: 2 }] };

    expect(narratorCues(view(before), view(healed))).toEqual([]);
    expect(narratorCues(view(werewolvesTurn), view(picked))).toEqual([]);
  });

  test("with Hidden Roles on, a dead Role's Turn is called and ends with its sleep line", () => {
    const deadSeer = [player(1, false, "seer"), ...[2, 3, 4, 5].map((id) => player(id))];

    expect(heard(narratorCues(view(vote, deadSeer, true), view(seersTurn, deadSeer, true)))).toEqual([
      "background:night",
      "nightfall",
      "seersTurn",
    ]);
    expect(heard(narratorCues(view(seersTurn, deadSeer, true), view(werewolvesTurn, deadSeer, true)))).toEqual([
      "seerSleeps",
      "sound:howl",
      "werewolvesTurn",
    ]);
  });

  test("every sleep line is the same whatever the Role did", () => {
    const passed: Moment = { type: "seersTurn", inspection: null, passed: true };
    const saw: Moment = { type: "seersTurn", inspection: { player: 2, role: "werewolf" }, passed: false };

    expect(narratorCues(view(passed), view(werewolvesTurn))).toEqual(narratorCues(view(saw), view(werewolvesTurn)));
  });

  test("every phone shows only the current Turn's wake line", () => {
    expect(narratorText(view(seersTurn))).toBe("La Voyante se réveille.");
    expect(narratorText(view(werewolvesTurn))).toBe("Les Loups-Garous se réveillent.");
    expect(narratorText(view(witchsTurn))).toBe("La Sorcière se réveille.");
  });
});

/** The cues for `moment` arriving after `before`, with the given Players. */
function cuesAt(moment: Moment, players = everyone, hiddenRoles = false, before: Moment = vote) {
  return narratorCues(view(before, players, hiddenRoles), view(moment, players, hiddenRoles));
}

describe("the dawn", () => {
  test("the last Role goes back to sleep, day breaks with the Day background, and every death is announced live", () => {
    const players = [player(1, false, "villager"), player(2, false, "seer"), player(3)];

    const cues = narratorCues(view(witchsTurn, players), view({ type: "dawn", deaths: [1, 2] }, players));

    expect(cues).toEqual([
      { type: "fixed", key: "witchSleeps", text: "La Sorcière range ses fioles et se rendort." },
      { type: "background", phase: "day" },
      { type: "fixed", key: "dawn", text: "Le jour se lève." },
      { type: "live", text: "Cette nuit, le village a perdu P1 (Villageois), P2 (Voyante)." },
    ]);
  });

  test("a death is announced without its cause", () => {
    const players = [player(1, false, "villager"), player(2)];

    expect(narratorText(view({ type: "dawn", deaths: [1] }, players))).not.toMatch(/poison|loup|sorci/i);
  });

  test("a Night without a death is all fixed lines", () => {
    expect(heard(cuesAt({ type: "dawn", deaths: [] }, everyone, false, witchsTurn))).toEqual([
      "witchSleeps",
      "background:day",
      "dawn",
      "dawnNobody",
    ]);
  });

  test("with Hidden Roles on, the dead are announced without their Role", () => {
    const players = [player(1, false), player(2, false), player(3)];

    expect(narratorText(view({ type: "dawn", deaths: [1, 2] }, players, true))).toBe(
      "Le jour se lève. Cette nuit, le village a perdu P1, P2.",
    );
  });

  test("with Hidden Roles on, a Table device that sees the Roles still never says them", () => {
    // A dead Host is a Spectator: their view reveals every Role.
    const players = [player(1, false, "seer"), player(2)];

    expect(narratorText(view({ type: "dawn", deaths: [1] }, players, true))).toBe(
      "Le jour se lève. Cette nuit, le village a perdu P1.",
    );
  });
});

describe("the Day", () => {
  test("the debate, the Election and the Vote are fixed lines", () => {
    expect(heard(cuesAt({ type: "discussion" }, everyone, false, { type: "dawn", deaths: [] }))).toEqual(["discussion"]);
    expect(heard(cuesAt({ type: "election", voted: [], yourBallot: null }, everyone, false, { type: "discussion" }))).toEqual([
      "election",
    ]);
    expect(heard(cuesAt(vote, everyone, false, { type: "discussion" }))).toEqual(["vote"]);
  });

  test("the Mayor's name is said live, and whether it was drawn", () => {
    expect(heard(cuesAt({ type: "electionResult", ballots: [], mayor: 2, byLot: false }))).toEqual(["P2 est élu Maire."]);
    expect(heard(cuesAt({ type: "electionResult", ballots: [], mayor: 2, byLot: true }))).toEqual([
      "Le sort désigne P2 comme Maire.",
    ]);
  });

  test("a tied Vote names the tied live, then hands it to the Mayor from MP3", () => {
    expect(heard(cuesAt({ type: "tieBreak", ballots: [], tied: [1, 2] }))).toEqual(["Égalité entre P1, P2.", "tieBreak"]);
  });

  test("an elimination by Vote lands with a sword slash and reveals the Role, unless it is hidden", () => {
    const voteResult: Moment = { type: "voteResult", ballots: [], eliminated: 2 };

    expect(heard(cuesAt(voteResult, [player(1), player(2, false, "witch")]))).toEqual([
      "sound:sword",
      "Le village a éliminé P2, qui était Sorcière.",
    ]);
    expect(heard(cuesAt(voteResult, [player(1), player(2, false)], true))).toEqual([
      "sound:sword",
      "Le village a éliminé P2.",
    ]);
  });

  test("a Vote that eliminates nobody is a fixed line, with no sword", () => {
    expect(heard(cuesAt({ type: "voteResult", ballots: [], eliminated: null }))).toEqual(["voteNobody"]);
  });

  test("the eliminated Hunter is named live, then called to shoot from MP3", () => {
    const players = [player(1, false, "hunter"), player(2), player(3)];

    expect(heard(cuesAt({ type: "huntersShot", hunter: 1 }, players))).toEqual([
      "P1, le Chasseur, emporte quelqu'un avec lui.",
      "huntersShot",
    ]);
  });

  test("the Hunter's shot lands with a gunshot and reveals its target's Role, unless it is hidden", () => {
    const shotResult: Moment = { type: "shotResult", hunter: 1, shot: 2 };

    expect(heard(cuesAt(shotResult, [player(1, false, "hunter"), player(2, false, "werewolf")]))).toEqual([
      "sound:gunshot",
      "Le Chasseur a tiré sur P2, qui était Loup-Garou.",
    ]);
    expect(heard(cuesAt(shotResult, [player(1, false), player(2, false)], true))).toEqual([
      "sound:gunshot",
      "Le Chasseur a tiré sur P2.",
    ]);
  });

  test("a Hunter who did not shoot in time fires no gunshot", () => {
    expect(heard(cuesAt({ type: "shotResult", hunter: 1, shot: null }))).toEqual(["shotLost"]);
  });

  test("the eliminated Mayor names a successor, or the lot does, live", () => {
    expect(heard(cuesAt({ type: "succession", mayor: 1 }))).toEqual(["P1, le Maire, désigne son successeur."]);
    expect(heard(cuesAt({ type: "successionResult", successor: 3, byLot: false }))).toEqual(["P3 devient Maire."]);
    expect(heard(cuesAt({ type: "successionResult", successor: 3, byLot: true }))).toEqual([
      "Le sort désigne P3 comme nouveau Maire.",
    ]);
  });

  test("the victory is a fixed line for each Camp", () => {
    expect(heard(cuesAt({ type: "victory", winner: "village" }))).toEqual(["villageWins"]);
    expect(heard(cuesAt({ type: "victory", winner: "werewolves" }))).toEqual(["werewolvesWin"]);
  });
});

describe("Paused", () => {
  test("the Narrator is silent while the Game is Paused", () => {
    expect(narratorCues(view(seersTurn), view(werewolvesTurn, everyone, false, true))).toEqual([]);
  });

  test("on resume, the current Moment's line is said again, with its background", () => {
    const paused = view(werewolvesTurn, everyone, false, true);

    expect(heard(narratorCues(paused, view(werewolvesTurn)))).toEqual(["background:night", "werewolvesTurn"]);
  });

  test("the Table device taking its seat back says the current Moment's line again", () => {
    expect(heard(narratorCues(null, view(vote)))).toEqual(["background:day", "vote"]);
    expect(heard(narratorCues(null, view(roleReveal)))).toEqual(["roleReveal"]);
  });
});

describe("the victory", () => {
  test("a victory at the end of a Turn still sends the Role back to sleep, at daybreak", () => {
    expect(heard(narratorCues(view(witchsTurn), view({ type: "victory", winner: "werewolves" })))).toEqual([
      "witchSleeps",
      "background:day",
      "werewolvesWin",
    ]);
  });
});

describe("resuming", () => {
  test("a Moment's one-off sound marks its start, so it is not played again on resume", () => {
    expect(heard(narratorCues(null, view(witchsTurn)))).toEqual(["background:night", "witchsTurn"]);
  });
});
