import { afterEach, describe, expect, test, vi } from "vitest";
import { rememberedMicMode, rememberMicMode } from "./seat";

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
