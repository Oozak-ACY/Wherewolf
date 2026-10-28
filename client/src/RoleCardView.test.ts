import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test } from "vitest";
import type { Moment } from "./generated/Moment";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCard } from "./generated/RoleCard";
import { allies, cardAction, cardImageUrl, cardOpensByItself, type Face, RoleCardView } from "./RoleCardView";
import { fr } from "./strings";

describe("the Role card opening by itself", () => {
  test("when the Roles are dealt", () => {
    expect(cardOpensByItself(null, "seer")).toBe(true);
  });

  test("not while the card stays the same", () => {
    expect(cardOpensByItself("seer", "seer")).toBe(false);
  });

  test("after the Theft, for the Thief holding the robbed Role and for the robbed Player holding a Villager card", () => {
    expect(cardOpensByItself("thief", "seer")).toBe(true);
    expect(cardOpensByItself("seer", "villager")).toBe(true);
  });

  test("not back in the Lobby", () => {
    expect(cardOpensByItself("seer", null)).toBe(false);
  });
});

describe("the Role card face down then face up", () => {
  const werewolf: RoleCard = { role: "werewolf", camp: "werewolves", fellowWerewolves: [2], potions: null };
  const players = ["Alice", "Bob", "Chloé"].map((name, i) => ({
    id: i + 1,
    name,
    connected: true,
    alive: true,
    revealedRole: null,
  }));
  const roleReveal = (confirmed: boolean) =>
    ({ type: "roleReveal", confirmed, stillReading: 3 }) as unknown as Moment;
  const theft = { type: "thief" } as unknown as Moment;

  function view(moment: Moment, extra: Partial<PlayerView> = {}): PlayerView {
    return { you: 1, players, role: werewolf, lovers: [3, 1], moment, spectating: false, ...extra } as PlayerView;
  }

  function render(face: Face, v: PlayerView): string {
    return renderToStaticMarkup(
      createElement(RoleCardView, {
        card: werewolf,
        view: v,
        face,
        onReveal: () => {},
        onConfirm: () => {},
        onClose: () => {},
      }),
    ).replaceAll("&#x27;", "'");
  }

  test("face down when the Roles are dealt, shows nothing of the Role and only reveals it", () => {
    const v = view(roleReveal(false));
    const html = render("down", v);
    for (const text of [
      fr.roles.werewolf.name,
      fr.roles.werewolf.power,
      fr.camps.werewolves.name,
      fr.camps.werewolves.goal,
      fr.roleCard.fellowWerewolves,
      "Bob",
      fr.roleCard.partner,
      "Chloé",
      "/roles/werewolf.webp",
    ]) {
      expect(html).not.toContain(text);
    }
    expect(html).toContain(fr.game.revealRole);
    expect(html).not.toContain(fr.game.confirmRole);
    expect(cardAction("down", v)).toBe("reveal");
  });

  test("face up during the Role reveal, shows the whole card and confirms the Role", () => {
    const v = view(roleReveal(false));
    const html = render("up", v);
    for (const text of [fr.roles.werewolf.name, fr.roles.werewolf.power, fr.camps.werewolves.name, "Bob", "Chloé"]) {
      expect(html).toContain(text);
    }
    expect(html).toContain(fr.game.confirmRole);
    expect(html).not.toContain(fr.game.revealRole);
    expect(cardAction("up", v)).toBe("confirm");
  });

  test("after the Theft, opens face down then only closes once revealed", () => {
    const v = view(theft);
    expect(cardAction("down", v)).toBe("reveal");
    expect(cardAction("up", v)).toBe("close");
    expect(render("up", v)).toContain(fr.roleCard.close);
  });

  test("reopened from Ma carte, shows the whole card straight away", () => {
    const html = render("up", view(theft));
    expect(html).toContain(fr.roles.werewolf.name);
    expect(html).not.toContain(fr.game.revealRole);
  });

  test("closes once this Player has confirmed their Role", () => {
    expect(cardAction("up", view(roleReveal(true)))).toBe("close");
  });

  test("never confirms for a Spectator", () => {
    const v = view(roleReveal(false), { spectating: true });
    expect(cardAction("up", v)).toBe("close");
    expect(render("up", v)).not.toContain(fr.game.confirmRole);
  });
});

describe("the Assassin's card", () => {
  test("shows the Assassin art, the Assassin Camp and its goal", () => {
    const card: RoleCard = { role: "assassin", camp: "assassin", fellowWerewolves: [], potions: null };
    const view = {
      you: 1,
      players: [{ id: 1, name: "Alice", connected: true, alive: true, revealedRole: null }],
      role: card,
      lovers: [],
      moment: { type: "roleReveal", confirmed: false, stillReading: 1 },
      spectating: false,
    } as unknown as PlayerView;

    const html = renderToStaticMarkup(
      createElement(RoleCardView, { card, view, face: "up", onReveal: () => {}, onConfirm: () => {}, onClose: () => {} }),
    ).replaceAll("&#x27;", "'");

    for (const text of [
      fr.roles.assassin.name,
      fr.roles.assassin.power,
      fr.camps.assassin.name,
      fr.camps.assassin.goal,
      "/roles/assassin.webp",
    ]) {
      expect(html).toContain(text);
    }
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

  /** The card of Chloé holding `role` and her view with these `lovers` */
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
