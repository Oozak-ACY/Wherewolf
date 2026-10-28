import { describe, expect, test } from "vitest";
import { fr } from "./strings";

describe("the role count buttons", () => {
  test("agree with a feminine Role", () => {
    expect(fr.settings.more("witch")).toBe("Une Sorcière de plus");
    expect(fr.settings.less("littleGirl")).toBe("Une Petite Fille de moins");
    expect(fr.settings.more("seer")).toBe("Une Voyante de plus");
  });

  test("agree with a masculine Role", () => {
    expect(fr.settings.more("werewolf")).toBe("Un Loup-Garou de plus");
    expect(fr.settings.less("hunter")).toBe("Un Chasseur de moins");
    expect(fr.settings.more("assassin")).toBe("Un Assassin de plus");
  });
});
