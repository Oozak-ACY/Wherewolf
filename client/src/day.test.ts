import { describe, expect, test } from "vitest";
import { abstainers, endTag, listTag, mayorNote, nameList, outcome, votersFor, yourVote } from "./day";
import type { Ballot } from "./generated/Ballot";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";

const [ALICE, BOB, CHLOE, DAVID] = [1, 2, 3, 4];
const players: PlayerSummary[] = ["Alice", "Bob", "Chloé", "David"].map((name, i) => ({
  id: i + 1,
  name,
  connected: true,
  alive: true,
  revealedRole: null,
}));

/** The view of Chloé by Day, at `moment`. */
function view(moment: Moment, changes: Partial<PlayerView> = {}): PlayerView {
  return { you: CHLOE, host: ALICE, players, moment, mayor: null, ...changes } as PlayerView;
}

const discussion: Moment = { type: "discussion" };
const ballots: Ballot[] = [
  { voter: ALICE, designated: BOB },
  { voter: CHLOE, designated: BOB },
  { voter: BOB, designated: ALICE },
  { voter: DAVID, designated: null },
];

describe("the tag under a name in the Day's Player list", () => {
  test("is empty for a living Player", () => {
    expect(listTag(view(discussion), players[1]!)).toBe("");
  });

  test("names the Player themselves", () => {
    expect(listTag(view(discussion), players[2]!)).toBe("toi");
  });

  test("names a Spectator with their revealed Role", () => {
    const dead = { ...players[1]!, alive: false, revealedRole: "seer" as const };
    expect(listTag(view(discussion), dead)).toBe("Voyante · spectateur");
  });

  test("names a Spectator whose Role stays hidden", () => {
    expect(listTag(view(discussion), { ...players[1]!, alive: false })).toBe("spectateur");
  });

  test("gives a living Player's Role once a Spectator sees it", () => {
    expect(listTag(view(discussion), { ...players[1]!, revealedRole: "werewolf" })).toBe("Loup-Garou");
  });

  test("says when a Player is Disconnected", () => {
    expect(listTag(view(discussion), { ...players[2]!, connected: false })).toBe("toi · déconnecté");
  });
});

describe("who voted", () => {
  test("for a Player, in the order they voted", () => {
    expect(votersFor(ballots, BOB)).toEqual([ALICE, CHLOE]);
    expect(votersFor(ballots, DAVID)).toEqual([]);
  });

  test("for nobody", () => {
    expect(abstainers(ballots)).toEqual([DAVID]);
  });
});

describe("a list of voters", () => {
  test("names each one and this Player as themselves", () => {
    expect(nameList(view(discussion), [ALICE, CHLOE])).toBe("Alice, Chloé (toi)");
  });

  test("is nothing without voters", () => {
    expect(nameList(view(discussion), [])).toBeNull();
  });
});

describe("this Player's own vote", () => {
  test("names whom they designated", () => {
    expect(yourVote(view({ type: "vote", voted: [CHLOE], yourBallot: { voter: CHLOE, designated: BOB } }))).toBe(
      "Ton vote : Bob",
    );
  });

  test("says they abstained", () => {
    expect(yourVote(view({ type: "vote", voted: [CHLOE], yourBallot: { voter: CHLOE, designated: null } }))).toBe(
      "Tu t'abstiens.",
    );
  });

  test("names whom they elect", () => {
    expect(
      yourVote(view({ type: "election", voted: [CHLOE], yourBallot: { voter: CHLOE, designated: ALICE } })),
    ).toBe("Ton vote : Alice");
  });

  test("is nothing before they vote", () => {
    expect(yourVote(view({ type: "vote", voted: [], yourBallot: null }))).toBeNull();
    expect(yourVote(view(discussion))).toBeNull();
  });
});

describe("the note on how a tied Vote is settled", () => {
  const vote: Moment = { type: "vote", voted: [], yourBallot: null };

  test("names the Mayor", () => {
    expect(mayorNote(view(vote, { mayor: BOB }))).toBe("Bob est Maire : en cas d'égalité, c'est au Maire de trancher.");
  });

  test("tells the Mayor it is up to them", () => {
    expect(mayorNote(view(vote, { mayor: CHLOE }))).toBe("Tu es Maire : en cas d'égalité, c'est toi qui tranches.");
  });

  test("is nothing without a Mayor", () => {
    expect(mayorNote(view(vote))).toBeNull();
  });
});

describe("the outcome of the Game", () => {
  test("names the winning Camp with its pill", () => {
    expect(outcome("village")).toEqual({ title: "Le Village triomphe", camp: "Village" });
    expect(outcome("werewolves")).toEqual({ title: "Les Loups-Garous triomphent", camp: "Loups-Garous" });
    expect(outcome("lovers")).toEqual({ title: "Les Amoureux triomphent", camp: "Amoureux" });
  });

  test("is a draw without a pill when nobody is left alive", () => {
    expect(outcome(null)).toEqual({ title: "Personne n'a survécu", camp: null });
  });
});

describe("the tag under a name at the end of the Game", () => {
  const victory: Moment = { type: "victory", winner: "village" };

  test("gives the Role of a Player still alive", () => {
    expect(endTag(view(victory), { ...players[1]!, revealedRole: "seer" })).toBe("Voyante · en vie");
  });

  test("gives the Role of a dead Player", () => {
    expect(endTag(view(victory), { ...players[1]!, alive: false, revealedRole: "werewolf" })).toBe("Loup-Garou · mort");
  });

  test("names the Player themselves", () => {
    expect(endTag(view(victory), { ...players[2]!, revealedRole: "villager" })).toBe("toi · Villageois · en vie");
  });
});
