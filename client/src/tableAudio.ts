// The Table device's audio: Web Audio for the Narrator's MP3s and the live
// lines the server voices (iOS ignores an `<audio>` element's volume), browser
// speech for everything else.

import { chooseFrenchVoice, unlockSpeech } from "./narrator";
import { isRecorded, recordedLines } from "./narratorRecordings";
import type { FixedLine } from "./narratorScript";
import type { NarratorVoiceDeps } from "./narratorVoice";
import { fetchLiveLine } from "./server";
import { fr } from "./strings";

let context: AudioContext | null = null;
const mp3s = new Map<FixedLine, AudioBuffer>();
let mp3Playing: AudioBufferSourceNode | null = null;

/**
 * Must run synchronously inside a tap, like the Host's on Start: iOS only lets
 * a page play sound later, without a tap, once it was unlocked during one.
 */
export function unlockTableAudio(): void {
  unlockSpeech();
  if (!("AudioContext" in window)) return;
  // On iOS, Web Audio otherwise follows the silent switch, while speech does not.
  const session = (navigator as { audioSession?: { type: string } }).audioSession;
  if (session) session.type = "playback";
  context ??= new AudioContext();
  void context.resume();
  // Older iOS only unlocks once something actually played.
  const silence = context.createBufferSource();
  silence.buffer = context.createBuffer(1, 1, 22050);
  silence.connect(context.destination);
  silence.start();
}

/** Loads every fixed line's MP3 that says its current text. Missing ones are spoken instead. */
export function preloadNarratorLines(): void {
  if (!("AudioContext" in window)) return;
  const audio = (context ??= new AudioContext());
  for (const [key, text] of Object.entries(fr.narrator.fixed) as [FixedLine, string][]) {
    if (mp3s.has(key) || !isRecorded(key, text, recordedLines)) continue;
    fetch(`${import.meta.env.BASE_URL}narrator/${key}.mp3`)
      .then((response) => (response.ok ? response.arrayBuffer() : Promise.reject(response.status)))
      .then((data) => audio.decodeAudioData(data))
      .then((buffer) => mp3s.set(key, buffer))
      .catch(() => {});
  }
}

function playMp3(key: FixedLine): Promise<void> | null {
  return play(mp3s.get(key));
}

function play(buffer: AudioBuffer | undefined): Promise<void> | null {
  if (!context || !buffer) return null;
  // A suspended context, never unlocked by a tap or interrupted by a call,
  // would never end the line: speech says it instead.
  if (context.state !== "running") {
    void context.resume();
    return null;
  }
  const source = context.createBufferSource();
  source.buffer = buffer;
  source.connect(context.destination);
  mp3Playing = source;
  const ended = new Promise<void>((resolve) => source.addEventListener("ended", () => resolve()));
  source.start();
  return ended;
}

function speak(text: string): Promise<void> {
  if (!("speechSynthesis" in window)) return Promise.resolve();
  const speech = window.speechSynthesis;
  const line = new SpeechSynthesisUtterance(text);
  line.lang = "fr-FR";
  // Looked up at each line: some browsers list their voices late.
  line.voice = chooseFrenchVoice(speech.getVoices());
  return new Promise((resolve) => {
    line.addEventListener("end", () => resolve());
    line.addEventListener("error", () => resolve());
    // Some browsers never say a line has ended: move on after a generous delay.
    setTimeout(resolve, 2000 + text.length * 150);
    speech.speak(line);
  });
}

function stop(): void {
  mp3Playing?.stop();
  mp3Playing = null;
  if ("speechSynthesis" in window) window.speechSynthesis.cancel();
}

/** The Table device's audio, with the live lines of the Lobby `code`. */
export function tableAudio(code: string): NarratorVoiceDeps {
  // Each live line asked of the server, by its text: asked once, even when
  // the line is said again (on resume) while it is still on its way.
  const requested = new Map<string, Promise<AudioBuffer>>();
  const voiced = new Map<string, AudioBuffer>();
  return {
    playMp3,
    speak,
    stop,
    async loadLive(text) {
      const audio = context;
      if (!audio) return false;
      let request = requested.get(text);
      if (!request) {
        request = fetchLiveLine(code, text).then((mp3) => audio.decodeAudioData(mp3));
        requested.set(text, request);
        // A failed line may be asked again, the next time it is said.
        request.catch(() => requested.delete(text));
      }
      voiced.set(text, await request);
      return true;
    },
    playLive: (text) => play(voiced.get(text)),
  };
}
