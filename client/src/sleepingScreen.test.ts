import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { Settings } from "./generated/Settings";
import { sleepingScreen, waitingScreen } from "./sleepingScreen";
import { fr } from "./strings";

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

/** What the engine tells each Role during each Night Turn, from the Turn's own Roles. */
const nightTurns: Record<"seersTurn" | "werewolvesTurn" | "witchsTurn", (role: Role) => Moment> = {
  seersTurn: (role) => ({
    type: "seersTurn",
    inspection: role === "seer" ? { player: BOB, role: "werewolf" } : null,
    passed: false,
  }),
  werewolvesTurn: (role) => ({
    type: "werewolvesTurn",
    picks: role === "werewolf" ? [{ werewolf: ALICE, victim: BOB }] : null,
  }),
  witchsTurn: (role) => ({
    type: "witchsTurn",
    witch: role === "witch" ? { victim: BOB, healed: false, poisoned: null, passed: false } : null,
  }),
} as Record<"seersTurn" | "werewolvesTurn" | "witchsTurn", (role: Role) => Moment>;

/** The view of `you`, holding `role`, at `moment`. */
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
    ...changes,
  };
}

const actingRole = { seersTurn: "seer", werewolvesTurn: "werewolf", witchsTurn: "witch" } as const;
const roles: Role[] = ["villager", "hunter", "littleGirl", "seer", "werewolf", "witch"];

describe("a phone at Night", () => {
  for (const [turn, moment] of Object.entries(nightTurns)) {
    const sleepers = roles.filter((r) => r !== actingRole[turn as keyof typeof actingRole]);

    test(`during the ${turn}, every non-acting Player sees the same sleeping screen`, () => {
      const screens = sleepers.map((role) => sleepingScreen(view(role, moment(role))));

      expect(screens[0]).not.toBeNull();
      for (const screen of screens) expect(screen).toEqual(screens[0]);
    });

    test(`during the ${turn}, the acting Role is awake`, () => {
      const role = actingRole[turn as keyof typeof actingRole];

      expect(sleepingScreen(view(role, moment(role)))).toBeNull();
    });
  }

  test("during the Thief's Turn, the Thief is awake and everyone else sees the same sleeping screen", () => {
    const thiefsTurn: Moment = { type: "thiefsTurn" };
    const screens = roles.map((role) => sleepingScreen(view(role, thiefsTurn)));

    expect(sleepingScreen(view("thief", thiefsTurn))).toBeNull();
    expect(screens[0]).not.toBeNull();
    for (const screen of screens) expect(screen).toEqual(screens[0]);
  });

  test("during Cupid's Turn, Cupid is awake and everyone else sees the same sleeping screen", () => {
    const cupidsTurn: Moment = { type: "cupidsTurn" };
    const screens = roles.map((role) => sleepingScreen(view(role, cupidsTurn)));

    expect(sleepingScreen(view("cupid", cupidsTurn))).toBeNull();
    expect(screens[0]).not.toBeNull();
    for (const screen of screens) expect(screen).toEqual(screens[0]);
  });

  test("during the Lovers' Turn, both Lovers are awake whatever their Role, and Cupid sleeps", () => {
    const loversTurn: Moment = { type: "loversTurn", confirmed: false };
    const lover = { lovers: [CHLOE, BOB] };
    const sleepers = [...roles, "cupid" as Role].map((role) => sleepingScreen(view(role, loversTurn)));

    for (const role of roles) expect(sleepingScreen(view(role, loversTurn, lover))).toBeNull();
    expect(sleepers[0]).not.toBeNull();
    for (const screen of sleepers) expect(screen).toEqual(sleepers[0]);
  });

  test("while the Lovers' Turn is announced, the Lovers still sleep like everyone else", () => {
    const loversTurn: Moment = { type: "loversTurn", confirmed: false };

    expect(sleepingScreen(view("villager", loversTurn, { lovers: [CHLOE, BOB], announcing: 4 }))).toEqual(
      sleepingScreen(view("villager", loversTurn)),
    );
  });

  test("while a Turn is announced, its acting Role still sleeps like everyone else", () => {
    const announced = { announcing: 4 };
    const screens = roles.map((role) => sleepingScreen(view(role, nightTurns.seersTurn(role), announced)));

    expect(screens[0]).not.toBeNull();
    for (const screen of screens) expect(screen).toEqual(screens[0]);
  });

  test("the sleeping screen shows the Narrator's line, never a Role", () => {
    const screen = sleepingScreen(view("villager", nightTurns.werewolvesTurn("villager")));

    expect(screen).toEqual({ line: fr.narrator.fixed.werewolvesTurn, pausedBy: null });
  });

  test("a dead Player, now a Spectator, sees the whole Night", () => {
    const dead = players.map((p) => (p.id === CHLOE ? { ...p, alive: false } : p));

    expect(
      sleepingScreen(view("villager", nightTurns.werewolvesTurn("werewolf"), { players: dead, spectating: true })),
    ).toBeNull();
  });

  test("someone who joined after the start, a Spectator, sees the whole Night", () => {
    expect(sleepingScreen(view(null, nightTurns.witchsTurn("witch")))).toBeNull();
  });

  test("a Paused Game says so on the sleeping screen, whatever the Role", () => {
    const screens = (["villager", "seer"] as Role[]).map((role) =>
      sleepingScreen(view(role, nightTurns.witchsTurn(role), { paused: true })),
    );

    expect(screens[0]?.pausedBy).toBe("Alice");
    expect(screens[1]).toEqual(screens[0]);
  });
});

describe("a phone outside the Night", () => {
  const moments: Moment[] = [
    { type: "roleReveal", stillReading: 2, confirmed: true },
    { type: "dawn", deaths: [], robbery: false },
    { type: "discussion" },
    { type: "vote", voted: [], yourBallot: null },
    { type: "victory", winner: "village" },
  ];

  for (const moment of moments) {
    test(`is awake at the ${moment.type}`, () => {
      expect(sleepingScreen(view("villager", moment))).toBeNull();
    });
  }
});

describe("a phone while the Narrator announces a Moment", () => {
  const discussion: Moment = { type: "discussion" };

  test("shows the same neutral waiting screen, whatever the Role", () => {
    const screens = [...roles, null].map((role) => waitingScreen(view(role, discussion, { announcing: 4 })));

    expect(screens[0]).toEqual({ line: fr.game.narratorSpeaking, pausedBy: null });
    for (const screen of screens) expect(screen).toEqual(screens[0]);
  });

  test("never says what is coming", () => {
    const vote: Moment = { type: "vote", voted: [], yourBallot: null };

    expect(waitingScreen(view("villager", vote, { announcing: 4 }))).toEqual(
      waitingScreen(view("villager", discussion, { announcing: 4 })),
    );
  });

  test("shows a Spectator the waiting screen at Night too", () => {
    expect(waitingScreen(view(null, nightTurns.seersTurn("seer"), { announcing: 4 }))).not.toBeNull();
  });

  test("says when the Game is Paused", () => {
    expect(waitingScreen(view("villager", discussion, { announcing: 4, paused: true }))?.pausedBy).toBe("Alice");
  });

  test("is gone once the Moment opens", () => {
    expect(waitingScreen(view("villager", discussion))).toBeNull();
  });
});
