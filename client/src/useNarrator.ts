import { useEffect, useRef } from "react";
import { narratorCues, type NarratorView } from "./narratorScript";
import { createNarratorVoice } from "./narratorVoice";
import { preloadNarratorLines, tableAudio } from "./tableAudio";

/**
 * Plays the Narrator's cues at each change of `view`, when `aloud`: only on
 * the Table device, which has the live lines voiced for the Lobby `code`.
 * Lines stay on screen either way. A new count of `seatings` says the current
 * line again: the Table device taking its seat back resumes a Paused Game.
 */
export function useNarrator(view: NarratorView, code: string, aloud: boolean, seatings: number) {
  const voice = useRef<ReturnType<typeof createNarratorVoice> | null>(null);
  // The view the last cues followed on, `null` when there is none to follow.
  const previous = useRef<NarratorView | null>(null);
  const seated = useRef(seatings);

  useEffect(() => {
    if (!aloud) return;
    preloadNarratorLines();
    voice.current = createNarratorVoice(tableAudio(code));
    previous.current = null;
    return () => {
      voice.current?.stop();
      voice.current = null;
    };
  }, [aloud, code]);

  useEffect(() => {
    if (!voice.current) return;
    if (seated.current !== seatings) {
      seated.current = seatings;
      previous.current = null;
    }
    const cues = narratorCues(previous.current, view);
    previous.current = view;
    if (cues.length > 0) voice.current.play(cues);
  }, [view, aloud, seatings]);
}
