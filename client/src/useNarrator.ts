import { useEffect, useRef } from "react";
import type { MomentId } from "./generated/MomentId";
import type { PlayerView } from "./generated/PlayerView";
import { narratorCues, type NarratorView } from "./narratorScript";
import { createNarratorVoice } from "./narratorVoice";
import { preloadTableAudio, tableAudio } from "./tableAudio";

/**
 * Plays the Narrator's cues at each change of `view`, when `aloud`: only on
 * the Table device, which has the live lines voiced for the Lobby `code`.
 * Once it has said every line announcing a Moment, it calls `onAnnounced`
 * so that Moment opens. Lines stay on screen either way. A new count of
 * `seatings` says the current line again: the Table device taking its seat
 * back resumes a Paused Game.
 */
export function useNarrator(
  view: NarratorView & Pick<PlayerView, "announcing">,
  code: string,
  aloud: boolean,
  seatings: number,
  onAnnounced: (moment: MomentId) => void,
) {
  const voice = useRef<ReturnType<typeof createNarratorVoice> | null>(null);
  // The view the last cues followed on, `null` when there is none to follow.
  const previous = useRef<NarratorView | null>(null);
  const seated = useRef(seatings);
  // The Moment whose lines are playing, or that was said to be announced.
  const handledMoment = useRef<MomentId | null>(null);
  // The Moment the latest view says is being announced: a Moment the backstop
  // has opened meanwhile is never said to be announced, late.
  const latestAnnouncing = useRef<MomentId | null>(null);
  const onAnnouncedRef = useRef(onAnnounced);
  onAnnouncedRef.current = onAnnounced;

  useEffect(() => {
    if (!aloud) return;
    preloadTableAudio();
    const audio = tableAudio(code);
    voice.current = createNarratorVoice(audio);
    previous.current = null;
    return () => {
      voice.current?.stop();
      voice.current = null;
      audio.setBackground(null);
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
    // Nobody can open a Moment while the Game is Paused.
    const moment = view.paused ? null : view.announcing;
    latestAnnouncing.current = moment;
    const announce = (moment: MomentId) => {
      if (latestAnnouncing.current === moment) onAnnouncedRef.current(moment);
    };
    if (cues.length > 0) {
      voice.current.play(cues, moment === null ? undefined : () => announce(moment));
      handledMoment.current = moment;
    } else if (moment !== null && handledMoment.current !== moment) {
      // Nothing new to say about this Moment: it opens at once.
      handledMoment.current = moment;
      announce(moment);
    }
  }, [view, aloud, seatings]);
}
