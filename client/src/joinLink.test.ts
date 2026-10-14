import { describe, expect, test } from "vitest";
import { joinLink, lobbyCodeFromPath } from "./joinLink";

describe("the join link", () => {
  test("points at the Lobby on this site", () => {
    expect(joinLink("https://wherewolf.app", "AB12")).toBe("https://wherewolf.app/l/AB12");
  });

  test("opens the join screen of that Lobby", () => {
    const link = new URL(joinLink("https://wherewolf.app", "AB12"));

    expect(lobbyCodeFromPath(link.pathname)).toBe("AB12");
  });

  test("still works typed in lowercase or with a trailing slash", () => {
    expect(lobbyCodeFromPath("/l/ab12/")).toBe("AB12");
  });

  test("is not a Lobby anywhere else", () => {
    expect(lobbyCodeFromPath("/")).toBeUndefined();
    expect(lobbyCodeFromPath("/l/")).toBeUndefined();
    expect(lobbyCodeFromPath("/l/AB-12")).toBeUndefined();
  });
});
