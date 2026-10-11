import { useEffect, useRef } from "react";
import type { NarratorSpeaking } from "./micControl";
import { chooseFrenchVoice, createNarrator } from "./narrator";

/**
 * Reads `line` aloud each time it changes, telling `speaking` while it plays
 * so this Player's mic stays muted. The line stays on screen either way: a
 * phone on silent mode only shows it.
 */
export function useNarrator(line: string | null, speaking: NarratorSpeaking) {
  const narrator = useRef<ReturnType<typeof createNarrator> | null>(null);

  useEffect(() => {
    if (!("speechSynthesis" in window)) return;
    const speech = window.speechSynthesis;
    // Some browsers list their voices at once, others only after `voiceschanged`,
    // and some iOS versions never fire it: look again while there is none.
    let voice = chooseFrenchVoice(speech.getVoices());
    const onVoices = () => {
      voice = chooseFrenchVoice(speech.getVoices());
    };
    speech.addEventListener("voiceschanged", onVoices);
    narrator.current = createNarrator({
      speech,
      utterance: (text) => new SpeechSynthesisUtterance(text),
      speaking,
      voice: () => (voice ??= chooseFrenchVoice(speech.getVoices())),
    });
    return () => {
      speech.removeEventListener("voiceschanged", onVoices);
      narrator.current?.stop();
      narrator.current = null;
    };
  }, [speaking]);

  useEffect(() => {
    if (line) narrator.current?.say(line);
  }, [line]);
}
