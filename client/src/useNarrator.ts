import { useEffect, useRef } from "react";
import { chooseFrenchVoice, createNarrator, type Mic } from "./narrator";

/**
 * Reads `line` aloud each time it changes, muting `mic` while it plays. The
 * line stays on screen either way: a phone on silent mode only shows it.
 */
export function useNarrator(line: string | null, mic: Mic) {
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
      mic,
      voice: () => (voice ??= chooseFrenchVoice(speech.getVoices())),
    });
    return () => {
      speech.removeEventListener("voiceschanged", onVoices);
      narrator.current?.stop();
      narrator.current = null;
    };
  }, [mic]);

  useEffect(() => {
    if (line) narrator.current?.say(line);
  }, [line]);
}
