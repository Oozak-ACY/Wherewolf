import type { PlayerView } from "./generated/PlayerView";
import { speaksAloud } from "./narrator";
import { narratorText, type Phase } from "./narratorScript";
import { isNightTurn, sleepingScreen, waitingScreen } from "./sleepingScreen";

/** What the Narrator bar atop every in-Game screen shows. */
export type NarratorBar = {
  /** The Night during its Turns, the Day for everything else. */
  phase: Phase;
  /** The Narrator's current line, the same on every phone. */
  line: string;
  /** Whether "Ma carte" opens the Player's card: never on a phone that hides it. */
  card: boolean;
  /** Whether the bar opens the mixer: on the Table device only. */
  mixer: boolean;
};

/** The Narrator bar of this phone's screen. `null` in the Lobby. */
export function narratorBar(view: PlayerView): NarratorBar | null {
  if (!view.moment) return null;
  return {
    phase: isNightTurn(view.moment) ? "night" : "day",
    line: narratorText(view) ?? "",
    card: view.role !== null && !sleepingScreen(view) && !waitingScreen(view),
    mixer: speaksAloud(view),
  };
}
