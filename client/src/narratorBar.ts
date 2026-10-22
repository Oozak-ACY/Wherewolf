import type { PlayerView } from "./generated/PlayerView";
import { speaksAloud } from "./narrator";
import { narratorText, type Phase } from "./narratorScript";
import { isNightTurn, sleepingScreen, waitingScreen } from "./sleepingScreen";
import { fr } from "./strings";

/** What the Narrator bar atop every in-Game screen shows */
export type NarratorBar = {
  /** The Night during its Turns and the Day for everything else including the Role reveal */
  phase: Phase;
  /** The Phase and its number with Jour 2 following the second Night */
  label: string;
  /** The Narrator's current line the same on every phone */
  line: string;
  /** Whether the bar opens the Player's card and never on a phone that hides it */
  card: boolean;
  /** Whether the bar opens the mixer on the Table device only */
  mixer: boolean;
};

/** The Narrator bar of this phone's screen and `null` in the Lobby */
export function narratorBar(view: PlayerView): NarratorBar | null {
  if (!view.moment) return null;
  const phase = isNightTurn(view.moment) ? "night" : "day";
  return {
    phase,
    label: fr.narratorBar.label(phase, view.night),
    line: narratorText(view) ?? "",
    card: view.role !== null && !sleepingScreen(view) && !waitingScreen(view),
    mixer: speaksAloud(view),
  };
}
