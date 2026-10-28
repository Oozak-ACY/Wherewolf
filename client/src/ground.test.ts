import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { Settings } from "./generated/Settings";
import { ground, picking, themeColor } from "./ground";

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

/** Each Night Turn as seen by the Player acting in it */
const nightTurns: [Role, Moment, Partial<PlayerView>][] = [
  ["cupid", { type: "cupidsTurn" }, {}],
  ["villager", { type: "loversTurn", confirmed: false }, { lovers: [CHLOE, BOB] }],
  ["thief", { type: "thiefsTurn" }, {}],
  ["seer", { type: "seersTurn", inspection: null, passed: false }, {}],
  ["werewolf", { type: "werewolvesTurn", picks: [] }, {}],
  ["witch", { type: "witchsTurn", witch: { dying: [BOB], healed: null, poisoned: null, passed: false } }, {}],
];

describe("the ground", () => {
  test("is parchment in the Lobby", () => {
    expect(ground(view(null, null), false)).toBe("parchment");
  });

  test("is reveal during the Role reveal", () => {
    expect(ground(view("villager", { type: "roleReveal", stillReading: 2, confirmed: false }), true)).toBe("reveal");
  });

  test("is reveal while the card dialog is open", () => {
    expect(ground(view("villager", { type: "discussion" }), true)).toBe("reveal");
  });

  for (const [role, moment, changes] of nightTurns) {
    test(`is night for the Player acting in the ${moment.type}`, () => {
      expect(ground(view(role, moment, changes), false)).toBe("night");
    });
  }

  test("is night for a sleeping Player, even with the card dialog left open", () => {
    expect(ground(view("villager", { type: "seersTurn", inspection: null, passed: false }), true)).toBe("night");
  });

  test("is night for a Spectator at Night", () => {
    expect(ground(view(null, { type: "werewolvesTurn", picks: [] }), false)).toBe("night");
  });

  test("is night while the Narrator announces a Night Turn", () => {
    expect(ground(view(null, { type: "seersTurn", inspection: null, passed: false }, { announcing: 4 }), false)).toBe(
      "night",
    );
  });

  const dayMoments: Moment[] = [
    { type: "dawn", deaths: [], robbery: false },
    { type: "discussion" },
    { type: "election", voted: [], yourBallot: null },
    { type: "vote", voted: [], yourBallot: null },
    { type: "tieBreak", ballots: [], tied: [ALICE, BOB] },
    { type: "victory", winner: "village" },
  ];

  for (const moment of dayMoments) {
    test(`is parchment at the ${moment.type}`, () => {
      expect(ground(view("villager", moment), false)).toBe("parchment");
    });
  }

  test("is parchment while waiting for the Narrator by Day, even with the card dialog left open", () => {
    expect(ground(view("villager", { type: "vote", voted: [], yourBallot: null }, { announcing: 4 }), true)).toBe(
      "parchment",
    );
  });

  test("is parchment while waiting for the Narrator to open the Role reveal", () => {
    const roleReveal: Moment = { type: "roleReveal", stillReading: 5, confirmed: false };

    expect(ground(view("villager", roleReveal, { announcing: 1 }), true)).toBe("parchment");
  });
});

describe("the theme color", () => {
  test("is a dark color on the dark grounds and cobalt by Day", () => {
    expect(themeColor("parchment")).toBe("#1F3F6E");
    expect(themeColor("night")).toBe("#0A1220");
    expect(themeColor("reveal")).toBe("#1A1210");
  });
});

describe("the pick rows", () => {
  const confirmed: [string, Role | null, Moment, Partial<PlayerView>][] = [
    ["Cupid", "cupid", { type: "cupidsTurn" }, {}],
    ["the Thief", "thief", { type: "thiefsTurn" }, {}],
    ["the Seer", "seer", { type: "seersTurn", inspection: null, passed: false }, {}],
    [
      "the Witch's poison",
      "witch",
      { type: "witchsTurn", witch: { dying: [BOB], healed: null, poisoned: null, passed: false } },
      {},
    ],
    ["the Hunter", "hunter", { type: "huntersShot", hunter: CHLOE }, {}],
    ["the Mayor's successor", "villager", { type: "succession", mayor: CHLOE }, { mayor: CHLOE }],
    ["the Mayor's tie-break", "villager", { type: "tieBreak", ballots: [], tied: [ALICE, BOB] }, { mayor: CHLOE }],
  ];

  for (const [who, role, moment, changes] of confirmed) {
    test(`wait for a confirm step for ${who}`, () => {
      expect(picking(view(role, moment, changes))).toBe("confirm");
    });
  }

  const tapped: [string, Moment][] = [
    ["the Werewolves", { type: "werewolvesTurn", picks: [] }],
    ["the Election", { type: "election", voted: [], yourBallot: null }],
    ["the Vote", { type: "vote", voted: [], yourBallot: null }],
  ];

  for (const [who, moment] of tapped) {
    test(`send on tap for ${who}`, () => {
      expect(picking(view("werewolf", moment))).toBe("tap");
    });
  }

  const none: [string, PlayerView][] = [
    ["in the Lobby", view(null, null)],
    ["in the debate", view("villager", { type: "discussion" })],
    ["to a Spectator", view(null, { type: "vote", voted: [], yourBallot: null })],
    ["to another Role in a Night Turn", view("villager", { type: "thiefsTurn" })],
    ["to the Seer once she has seen", view("seer", { type: "seersTurn", inspection: { player: BOB, role: "witch" }, passed: false })],
    ["to the Seer once she has passed", view("seer", { type: "seersTurn", inspection: null, passed: true })],
    [
      "to the Witch without her poison",
      view(
        "witch",
        { type: "witchsTurn", witch: { dying: [BOB], healed: null, poisoned: null, passed: false } },
        { role: { role: "witch", camp: "village", fellowWerewolves: [], potions: { healing: true, poison: false } } },
      ),
    ],
    [
      "to the Witch once she has passed",
      view("witch", { type: "witchsTurn", witch: { dying: [BOB], healed: null, poisoned: null, passed: true } }),
    ],
    ["to anyone but the Hunter", view("villager", { type: "huntersShot", hunter: BOB })],
    ["to anyone but the Mayor at a tie-break", view("villager", { type: "tieBreak", ballots: [], tied: [ALICE, BOB] }, { mayor: BOB })],
    ["to anyone but the dying Mayor", view("villager", { type: "succession", mayor: BOB }, { mayor: BOB })],
    ["while the Narrator announces the Moment", view("villager", { type: "vote", voted: [], yourBallot: null }, { announcing: 4 })],
    [
      "to a dead Player",
      view("villager", { type: "vote", voted: [], yourBallot: null }, {
        players: players.map((p) => (p.id === CHLOE ? { ...p, alive: false } : p)),
      }),
    ],
  ];

  for (const [when, seen] of none) {
    test(`are not offered ${when}`, () => {
      expect(picking(seen)).toBeNull();
    });
  }
});
