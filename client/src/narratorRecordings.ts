// The MP3s of the Narrator's fixed lines, and the text each one was generated
// from. Imported by scripts/generate-narrator-lines.ts too, so it imports
// nothing Node could not load.

import recorded from "./narratorRecordings.json" with { type: "json" };

/** Each fixed line's key, mapped to the text its committed MP3 says. */
export type Recordings = Record<string, string>;

export const recordedLines: Recordings = recorded;

/** Whether the fixed line `key` can play from its MP3: one exists, and it says `text`. */
export function isRecorded(key: string, text: string, recordings: Recordings): boolean {
  return recordings[key] === text;
}

export type GenerationDeps<Mp3> = {
  /** Speaks `text` with the Narrator's voice. */
  synthesize: (text: string) => Promise<Mp3>;
  write: (key: string, mp3: Mp3) => Promise<void>;
  remove: (key: string) => Promise<void>;
};

/**
 * Brings the MP3s in line with `lines`, each fixed line's key mapped to its
 * text: generates the new and changed ones, removes those whose line is gone.
 * Returns the new recordings.
 */
export async function generateNarratorLines<Mp3>(
  lines: Recordings,
  recordings: Recordings,
  { synthesize, write, remove }: GenerationDeps<Mp3>,
): Promise<Recordings> {
  const next: Recordings = {};
  for (const [key, text] of Object.entries(lines)) {
    if (!isRecorded(key, text, recordings)) await write(key, await synthesize(text));
    next[key] = text;
  }
  for (const key of Object.keys(recordings)) {
    if (!(key in lines)) await remove(key);
  }
  return next;
}
