import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { Settings } from "./generated/Settings";
import { nightTurn } from "./nightTurn";

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
function view(role: Role | null, moment: Moment, changes: Partial<PlayerView> = {}): PlayerView {
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
    spectating: role === null,
    moment,
    mayor: null,
    lovers: [],
    theft: null,
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

/** Each Night Turn with the Player acting in it and the card and title atop its screen */
const turns: [Moment, Role, Partial<PlayerView>, { card: string; title: string }][] = [
  [{ type: "cupidsTurn" }, "cupid", {}, { card: "cupid", title: "Cupidon" }],
  [{ type: "loversTurn", confirmed: false }, "villager", { lovers: [CHLOE, BOB] }, { card: "cupid", title: "Amoureux" }],
  [{ type: "thiefsTurn" }, "thief", {}, { card: "thief", title: "Voleur" }],
  [{ type: "seersTurn", inspection: null, passed: false }, "seer", {}, { card: "seer", title: "Voyante" }],
  [
    { type: "werewolvesTurn", picks: [{ werewolf: CHLOE, victim: null }] },
    "werewolf",
    {},
    { card: "werewolf", title: "Loups-Garous" },
  ],
  [
    { type: "witchsTurn", witch: { dying: [BOB], healed: null, poisoned: null, passed: false } },
    "witch",
    {},
    { card: "witch", title: "Sorcière" },
  ],
];

describe("a Night Turn's screen", () => {
  for (const [moment, role, changes, expected] of turns) {
    test(`shows the acting Player the ${moment.type} card and title`, () => {
      expect(nightTurn(view(role, moment, changes))).toEqual(expected);
    });

    test(`shows a Spectator the same ${moment.type} card and title`, () => {
      expect(nightTurn(view(null, moment))).toEqual(expected);
    });
  }

  test("is not shown on a sleeping phone", () => {
    expect(nightTurn(view("villager", { type: "seersTurn", inspection: null, passed: false }))).toBeNull();
  });

  test("is not shown while the Narrator announces the Turn", () => {
    expect(nightTurn(view("seer", { type: "seersTurn", inspection: null, passed: false }, { announcing: 4 }))).toBeNull();
  });

  test("is not shown by Day", () => {
    expect(nightTurn(view("seer", { type: "discussion" }))).toBeNull();
    expect(nightTurn(view(null, { type: "vote", voted: [], yourBallot: null }))).toBeNull();
  });
});
