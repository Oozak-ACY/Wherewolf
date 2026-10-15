// Generates the MP3 of each fixed Narrator line (fr.narrator.fixed in
// src/strings.ts) with ElevenLabs, into public/narrator/<key>.mp3, and records
// the text each one says in src/narratorRecordings.json. Only new and changed
// lines are generated, unless --all is passed (after changing the voice).
//
//   ELEVENLABS_API_KEY=… ELEVENLABS_VOICE_ID=… npm run generate-narrator-lines
//
// The voice is "Narrateur sombre", made with Voice Design (prompt in the
// prototype/narrator-voice branch's README). Commit the MP3s and the JSON.

import { mkdir, rm, writeFile } from "node:fs/promises";
import { generateNarratorLines, recordedLines } from "../src/narratorRecordings.ts";
import { fr } from "../src/strings.ts";

const MODEL = "eleven_v3";
const mp3Dir = new URL("../public/narrator/", import.meta.url);
const recordingsFile = new URL("../src/narratorRecordings.json", import.meta.url);

const apiKey = process.env.ELEVENLABS_API_KEY;
const voiceId = process.env.ELEVENLABS_VOICE_ID;
if (!apiKey || !voiceId) {
  console.error("Set ELEVENLABS_API_KEY and ELEVENLABS_VOICE_ID (the \"Narrateur sombre\" voice).");
  process.exit(1);
}

async function synthesize(text: string): Promise<Uint8Array> {
  console.log(`Generating: ${text}`);
  const response = await fetch(
    `https://api.elevenlabs.io/v1/text-to-speech/${voiceId}?output_format=mp3_44100_128`,
    {
      method: "POST",
      headers: { "content-type": "application/json", "xi-api-key": apiKey! },
      body: JSON.stringify({ text, model_id: MODEL }),
    },
  );
  if (!response.ok) throw new Error(`ElevenLabs answered ${response.status}: ${await response.text()}`);
  return new Uint8Array(await response.arrayBuffer());
}

await mkdir(mp3Dir, { recursive: true });
const all = process.argv.includes("--all");
const recordings = await generateNarratorLines(fr.narrator.fixed, all ? {} : recordedLines, {
  synthesize,
  write: (key, mp3) => writeFile(new URL(`${key}.mp3`, mp3Dir), mp3),
  remove: (key) => rm(new URL(`${key}.mp3`, mp3Dir), { force: true }),
});
await writeFile(recordingsFile, `${JSON.stringify(recordings, null, 2)}\n`);
console.log(`${Object.keys(recordings).length} fixed lines recorded.`);
