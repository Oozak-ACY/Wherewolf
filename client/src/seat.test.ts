import { afterEach, describe, expect, test, vi } from "vitest";
import { DEFAULT_LEVELS, MAX_LEVEL } from "./mixer";
import { rememberedLevels, rememberedMicMode, rememberLevels, rememberMicMode } from "./seat";

function memoryStorage() {
  const items = new Map<string, string>();
  return {
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => void items.set(key, value),
    removeItem: (key: string) => void items.delete(key),
  };
}

afterEach(() => vi.unstubAllGlobals());

describe("the mic mode", () => {
  test("is push-to-talk for a Player who never chose", () => {
    vi.stubGlobal("localStorage", memoryStorage());

    expect(rememberedMicMode()).toBe("pushToTalk");
  });

  test("survives a reload on the same browser", () => {
    vi.stubGlobal("localStorage", memoryStorage());

    rememberMicMode("open");

    expect(rememberedMicMode()).toBe("open");
  });

  test("falls back to push-to-talk when storage is unavailable", () => {
    const blocked = () => {
      throw new DOMException("blocked", "SecurityError");
    };
    vi.stubGlobal("localStorage", { getItem: blocked, setItem: blocked, removeItem: blocked });

    rememberMicMode("open");

    expect(rememberedMicMode()).toBe("pushToTalk");
  });

  test("ignores a stored value it does not know", () => {
    const storage = memoryStorage();
    storage.setItem("wherewolf.micMode", "shout");
    vi.stubGlobal("localStorage", storage);

    expect(rememberedMicMode()).toBe("pushToTalk");
  });
});

describe("the Host's mixer levels", () => {
  test("are the defaults on a phone that never set them", () => {
    vi.stubGlobal("localStorage", memoryStorage());

    expect(rememberedLevels()).toEqual(DEFAULT_LEVELS);
  });

  test("survive from one Game to the next on the same phone", () => {
    vi.stubGlobal("localStorage", memoryStorage());

    rememberLevels({ narrator: 1.5, ambiance: 0, effects: 0.45 });

    expect(rememberedLevels()).toEqual({ narrator: 1.5, ambiance: 0, effects: 0.45 });
  });

  test("fall back to the defaults when storage is unavailable", () => {
    const blocked = () => {
      throw new DOMException("blocked", "SecurityError");
    };
    vi.stubGlobal("localStorage", { getItem: blocked, setItem: blocked, removeItem: blocked });

    rememberLevels({ narrator: 1.5, ambiance: 0, effects: 0.45 });

    expect(rememberedLevels()).toEqual(DEFAULT_LEVELS);
  });

  test("keep only the levels that make sense", () => {
    const storage = memoryStorage();
    storage.setItem("wherewolf.mixerLevels", JSON.stringify({ narrator: 9, ambiance: "loud", effects: 0.3 }));
    vi.stubGlobal("localStorage", storage);

    expect(rememberedLevels()).toEqual({ narrator: MAX_LEVEL, ambiance: DEFAULT_LEVELS.ambiance, effects: 0.3 });
  });

  test("ignore a stored value that is not levels at all", () => {
    const storage = memoryStorage();
    storage.setItem("wherewolf.mixerLevels", "{not json");
    vi.stubGlobal("localStorage", storage);

    expect(rememberedLevels()).toEqual(DEFAULT_LEVELS);
  });
});
