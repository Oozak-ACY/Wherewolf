import { describe, expect, test } from "vitest";
import { generateNarratorLines, isRecorded, recordedLines } from "./narratorRecordings";
import { fr } from "./strings";

/** An MP3 store and a voice that only record what they were asked. */
function fakes() {
  const files = new Map<string, string>();
  const synthesized: string[] = [];
  return {
    files,
    synthesized,
    deps: {
      synthesize: async (text: string) => {
        synthesized.push(text);
        return `mp3 of ${text}`;
      },
      write: async (key: string, mp3: string) => void files.set(key, mp3),
      remove: async (key: string) => void files.delete(key),
    },
  };
}

describe("generating the fixed lines' MP3s", () => {
  test("writes one MP3 per fixed line key and records its source text", async () => {
    const { files, deps } = fakes();

    const recorded = await generateNarratorLines({ dawn: "Le jour se lève.", vote: "Le village vote." }, {}, deps);

    expect(Object.fromEntries(files)).toEqual({ dawn: "mp3 of Le jour se lève.", vote: "mp3 of Le village vote." });
    expect(recorded).toEqual({ dawn: "Le jour se lève.", vote: "Le village vote." });
  });

  test("only regenerates the lines whose text changed", async () => {
    const { synthesized, deps } = fakes();

    const recorded = await generateNarratorLines(
      { dawn: "Le jour se lève.", vote: "Le village vote, enfin." },
      { dawn: "Le jour se lève.", vote: "Le village vote." },
      deps,
    );

    expect(synthesized).toEqual(["Le village vote, enfin."]);
    expect(recorded).toEqual({ dawn: "Le jour se lève.", vote: "Le village vote, enfin." });
  });

  test("removes the MP3 of a line that no longer exists", async () => {
    const { files, deps } = fakes();
    files.set("old", "mp3 of an old line");

    const recorded = await generateNarratorLines({}, { old: "Une vieille réplique." }, deps);

    expect(files.has("old")).toBe(false);
    expect(recorded).toEqual({});
  });
});

describe("the committed MP3s", () => {
  test("each one was generated from its fixed line's current text", () => {
    const fixed: Record<string, string> = fr.narrator.fixed;

    for (const [key, text] of Object.entries(recordedLines)) {
      expect(fixed[key], `"${key}" has no fixed line any more: run npm run generate-narrator-lines`).toBeDefined();
      expect(text, `the MP3 of "${key}" is stale: run npm run generate-narrator-lines`).toBe(fixed[key]);
    }
  });

  test("a fixed line plays from MP3 only when its MP3 says the same text", () => {
    expect(isRecorded("dawn", "Le jour se lève.", { dawn: "Le jour se lève." })).toBe(true);
    expect(isRecorded("dawn", "Le jour se lève.", { dawn: "Le soleil se lève." })).toBe(false);
    expect(isRecorded("dawn", "Le jour se lève.", {})).toBe(false);
  });
});
