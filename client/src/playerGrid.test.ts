import { describe, expect, it } from "vitest";
import { gridSize } from "./PlayerGrid";

describe("gridSize", () => {
  it.each([
    [1, "small"],
    [4, "small"],
    [5, "medium"],
    [9, "medium"],
    [10, "large"],
    [12, "large"],
    [13, "crowded"],
    [14, "crowded"],
    [15, "crowded"],
  ])("gives %i Players a %s grid", (count, size) => {
    expect(gridSize(count)).toBe(size);
  });
});
