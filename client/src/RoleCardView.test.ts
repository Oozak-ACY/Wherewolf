import { describe, expect, test } from "vitest";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCard } from "./generated/RoleCard";
import { allies, cardTurnsFaceUp, cardImageUrl } from "./RoleCardView";
import { fr } from "./strings";

describe("the Role card turning face up by itself", () => {
  test("when the Roles are dealt", () => {
    expect(cardTurnsFaceUp(null, "seer")).toBe(true);
  });

  test("not while the card stays the same", () => {
    expect(cardTurnsFaceUp("seer", "seer")).toBe(false);
  });

  test("after the Theft, for the Thief holding the robbed Role and for the robbed Player holding a Villager card", () => {
    expect(cardTurnsFaceUp("thief", "seer")).toBe(true);
    expect(cardTurnsFaceUp("seer", "villager")).toBe(true);
  });

  test("not back in the Lobby", () => {
    expect(cardTurnsFaceUp("seer", null)).toBe(false);
  });
});

describe("the artwork of a card", () => {
  test("is the stained glass webp named after the Role", () => {
    expect(cardImageUrl("werewolf")).toBe("/roles/werewolf.webp");
  });

  test("of the Mayor title too", () => {
    expect(cardImageUrl("mayor")).toBe("/roles/mayor.webp");
  });
});

describe("the allies shown on the reveal", () => {
  const players = ["Alice", "Bob", "Chloé", "David"].map((name, i) => ({
    id: i + 1,
    name,
    connected: true,
    alive: true,
    revealedRole: null,
  }));

  /** The card of Chloé, holding `role`, and her view with these `lovers`. */
  function reveal(role: Role, fellowWerewolves: number[], lovers: number[] = []): [RoleCard, PlayerView] {
    const card: RoleCard = { role, camp: role === "werewolf" ? "werewolves" : "village", fellowWerewolves, potions: null };
    return [card, { you: 3, players, role: card, lovers } as unknown as PlayerView];
  }

  test("are the pack for a Werewolf", () => {
    expect(allies(...reveal("werewolf", [1, 4]))).toEqual([
      { title: fr.roleCard.fellowWerewolves, names: "Alice, David" },
    ]);
  });

  test("tell a lone Werewolf they are alone", () => {
    expect(allies(...reveal("werewolf", []))).toEqual([
      { title: null, names: fr.roleCard.loneWerewolf },
    ]);
  });

  test("are the partner for a Lover, whatever their Role", () => {
    expect(allies(...reveal("seer", [], [2, 3]))).toEqual([{ title: fr.roleCard.partner, names: "Bob" }]);
  });

  test("are both for a Werewolf in love", () => {
    expect(allies(...reveal("werewolf", [1], [3, 2]))).toEqual([
      { title: fr.roleCard.fellowWerewolves, names: "Alice" },
      { title: fr.roleCard.partner, names: "Bob" },
    ]);
  });

  test("are nobody for a Villager", () => {
    expect(allies(...reveal("villager", []))).toEqual([]);
  });
});
