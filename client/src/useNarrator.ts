import { useEffect, useRef } from "react";
import { chooseFrenchVoice, createNarrator } from "./narrator";

/**
 * Reads `line` aloud each time it changes, when `aloud`: only on the Table
 * device. The line stays on screen either way: other phones, and a phone on
 * silent mode, only show it.
 */
export function useNarrator(line: string | null, aloud: boolean) {
  const narrator = useRef<ReturnType<typeof createNarrator> | null>(null);

  useEffect(() => {
    if (!aloud || !("speechSynthesis" in window)) return;
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
      voice: () => (voice ??= chooseFrenchVoice(speech.getVoices())),
    });
    return () => {
      speech.removeEventListener("voiceschanged", onVoices);
      narrator.current?.stop();
      narrator.current = null;
    };
  }, [aloud]);

  useEffect(() => {
    if (line) narrator.current?.say(line);
  }, [line]);
}
