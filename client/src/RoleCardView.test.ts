import { describe, expect, test } from "vitest";
import { cardTurnsFaceUp, cardImageUrl } from "./RoleCardView";

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
