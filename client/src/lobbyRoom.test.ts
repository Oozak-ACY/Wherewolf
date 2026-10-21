import { describe, expect, test } from "vitest";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { RoleCounts } from "./generated/RoleCounts";
import { initial, playerTag, roleTotal } from "./lobbyRoom";

const seat = (id: number, name: string, connected = true): PlayerSummary => ({
  id,
  name,
  connected,
  alive: true,
  revealedRole: null,
});

const view = (you: number, host: number) => ({ you, host }) as PlayerView;

describe("a Player's tile tag", () => {
  test("names the Host and this Player", () => {
    expect(playerTag(view(1, 1), seat(1, "Camille"))).toBe("Hôte · toi");
    expect(playerTag(view(2, 1), seat(1, "Camille"))).toBe("Hôte");
    expect(playerTag(view(2, 1), seat(2, "Hugo"))).toBe("toi");
  });

  test("says anyone else is ready", () => {
    expect(playerTag(view(1, 1), seat(2, "Hugo"))).toBe("prêt");
  });

  test("says a Disconnected Player is disconnected", () => {
    expect(playerTag(view(1, 1), seat(2, "Mathis", false))).toBe("déconnecté");
    expect(playerTag(view(2, 1), seat(1, "Camille", false))).toBe("Hôte · déconnecté");
  });
});

describe("a Player's avatar", () => {
  test("shows the first letter of their name in capitals", () => {
    expect(initial("léa")).toBe("L");
  });

  test("keeps a whole emoji", () => {
    expect(initial("🐺 Loup")).toBe("🐺");
  });
});

test("the Role total adds up every Role in play", () => {
  const roles = { werewolf: 2, cupid: 0, thief: 0, seer: 1, witch: 1, hunter: 0, villager: 2 } as RoleCounts;

  expect(roleTotal(roles)).toBe(6);
});
