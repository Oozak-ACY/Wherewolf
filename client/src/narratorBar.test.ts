import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { Settings } from "./generated/Settings";
import { narratorBar } from "./narratorBar";
import { narratorText } from "./narratorScript";

const [ALICE, BOB, CHLOE] = [1, 2, 3];
const players: PlayerSummary[] = ["Alice", "Bob", "Chloé", "David", "Emma"].map((name, i) => ({
  id: i + 1,
  name,
  connected: true,
  alive: true,
  revealedRole: null,
}));

const settings = {
  roles: { werewolf: 1, seer: 1, witch: 1, hunter: 1, villager: 1 },
  hiddenRoles: false,
} as unknown as Settings;

/** The view of `you` holding `role` at `moment` */
function view(role: Role | null, moment: Moment | null, changes: Partial<PlayerView> = {}): PlayerView {
  return {
    code: "ABCD",
    you: CHLOE,
    host: ALICE,
    players,
    settings,
    suggestedRoles: settings.roles,
    startBlockedBy: null,
    role: role && {
      role,
      camp: role === "werewolf" ? "werewolves" : "village",
      fellowWerewolves: [],
      potions: role === "witch" ? { healing: true, poison: true } : null,
    },
    spectating: moment !== null && role === null,
    moment,
    mayor: null,
    lovers: [],
    theft: null,
    assassinTarget: null,
    receives: [],
    audience: [],
    seesCountdown: true,
    mayEndDebate: false,
    paused: false,
    announcing: null,
    night: 1,
    ...changes,
  };
}

const seersTurn: Moment = { type: "seersTurn", inspection: null, passed: false };
const vote: Moment = { type: "vote", voted: [], yourBallot: null };

describe("the Narrator bar", () => {
  test("names the Night by its number", () => {
    expect(narratorBar(view("villager", seersTurn, { night: 2 }))?.label).toBe("Nuit 2 · Le Narrateur");
  });

  test("names the Day after the Night it follows", () => {
    expect(narratorBar(view("villager", vote, { night: 1 }))?.label).toBe("Jour 1 · Le Narrateur");
    expect(narratorBar(view("villager", vote, { night: 2 }))?.label).toBe("Jour 2 · Le Narrateur");
  });

  test("names only the Narrator during the Role reveal, before any Night", () => {
    const roleReveal: Moment = { type: "roleReveal", stillReading: 5, confirmed: false };

    expect(narratorBar(view("villager", roleReveal, { night: 0 }))?.label).toBe("Le Narrateur");
  });

  test("is not shown in the Lobby", () => {
    expect(narratorBar(view(null, null))).toBeNull();
  });

  const nightTurns: Moment[] = [
    { type: "cupidsTurn" },
    { type: "loversTurn", confirmed: false },
    { type: "thiefsTurn" },
    seersTurn,
    { type: "werewolvesTurn", picks: null },
    { type: "witchsTurn", witch: null },
  ];

  for (const moment of nightTurns) {
    test(`is the Night's at the ${moment.type}`, () => {
      expect(narratorBar(view("villager", moment))?.phase).toBe("night");
    });
  }

  const dayMoments: Moment[] = [
    { type: "roleReveal", stillReading: 2, confirmed: false },
    { type: "dawn", deaths: [], robbery: false },
    { type: "discussion" },
    { type: "election", voted: [], yourBallot: null },
    vote,
    { type: "tieBreak", ballots: [], tied: [ALICE, BOB] },
    { type: "huntersShot", hunter: BOB },
    { type: "victory", winner: "village" },
  ];

  for (const moment of dayMoments) {
    test(`is the Day's at the ${moment.type}`, () => {
      expect(narratorBar(view("villager", moment))?.phase).toBe("day");
    });
  }

  test("says the Narrator's current line, the same on every phone", () => {
    const dawn: Moment = { type: "dawn", deaths: [BOB], robbery: false };

    expect(narratorBar(view("seer", dawn))?.line).toBe(narratorText(view(null, dawn)));
    expect(narratorBar(view("werewolf", dawn))?.line).toBe(narratorBar(view("seer", dawn))?.line);
  });

  test("says the line being announced while the Narrator speaks", () => {
    expect(narratorBar(view("villager", vote, { announcing: 4 }))?.line).toBe(narratorText(view(null, vote)));
  });

  test("opens the Player's card while they are awake", () => {
    expect(narratorBar(view("seer", seersTurn))?.card).toBe(true);
    expect(narratorBar(view("villager", vote))?.card).toBe(true);
  });

  test("does not open the card on a sleeping phone, which hides it", () => {
    expect(narratorBar(view("villager", seersTurn))?.card).toBe(false);
  });

  test("does not open the card while waiting for the Narrator", () => {
    expect(narratorBar(view("villager", vote, { announcing: 4 }))?.card).toBe(false);
  });

  test("does not open a card for someone who never held one", () => {
    expect(narratorBar(view(null, vote))?.card).toBe(false);
  });

  test("holds the mixer in the options menu on the Table device only", () => {
    expect(narratorBar(view("villager", vote, { you: ALICE }))?.mixer).toBe(true);
    expect(narratorBar(view("villager", seersTurn, { you: ALICE }))?.mixer).toBe(true);
    expect(narratorBar(view("villager", vote))?.mixer).toBe(false);
  });
});
