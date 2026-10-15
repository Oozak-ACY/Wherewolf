import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import { narratorText } from "./narratorScript";
import { fr } from "./strings";

/**
 * Everything a sleeping phone shows. Nothing in it depends on the Player's
 * Role, so a glance at any sleeping phone gives nothing away.
 */
export type SleepingScreen = {
  /** The Narrator's current line, the same on every phone. */
  line: string;
  /** The Host's name while the Game is Paused waiting for them. */
  pausedBy: string | null;
};

/** The Role that wakes up during each Night Turn. */
const ACTING: Partial<Record<NonNullable<PlayerView["moment"]>["type"], Role>> = {
  seersTurn: "seer",
  werewolvesTurn: "werewolf",
  witchsTurn: "witch",
};

/**
 * What this phone shows at Night if its Player is asleep: every living
 * Player but the acting Role's, and the acting Role's too while the Narrator
 * announces their Turn. `null` when the phone is awake: outside the Night,
 * during its Player's own Turn, or for a Spectator, who sees it all.
 */
export function sleepingScreen(view: PlayerView): SleepingScreen | null {
  const { moment } = view;
  if (!moment || view.spectating) return null;
  const acting = ACTING[moment.type];
  if (!acting) return null;
  const alive = view.players.find((p) => p.id === view.you)?.alive ?? false;
  if (alive && view.role?.role === acting && view.announcing === null) return null;
  return { line: narratorText(view) ?? "", pausedBy: pausedBy(view) };
}

/**
 * What a phone that isn't asleep shows while the Narrator announces the
 * current Moment: the same neutral screen on every phone, whatever comes
 * next. `null` once the Moment opens.
 */
export function waitingScreen(view: PlayerView): SleepingScreen | null {
  if (!view.moment || view.announcing === null) return null;
  return { line: fr.game.narratorSpeaking, pausedBy: pausedBy(view) };
}

/** The Host's name while the Game is Paused waiting for them. */
function pausedBy(view: PlayerView): string | null {
  return view.paused ? (view.players.find((p) => p.id === view.host)?.name ?? "?") : null;
}
