import type { Moment } from "./generated/Moment";
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

/**
 * Who wakes up during each Night Turn: the Players holding its Role, or for
 * the Lovers' Turn, the two Lovers whatever their Roles.
 */
const ACTING: Partial<Record<NonNullable<PlayerView["moment"]>["type"], (view: PlayerView) => boolean>> = {
  cupidsTurn: holds("cupid"),
  loversTurn: (view) => view.lovers.includes(view.you),
  thiefsTurn: holds("thief"),
  seersTurn: holds("seer"),
  werewolvesTurn: holds("werewolf"),
  assassinsTurn: holds("assassin"),
  witchsTurn: holds("witch"),
};

/** Whether `moment` is a Night Turn in which some Players act while the others sleep */
export function isNightTurn(moment: Moment): boolean {
  return moment.type in ACTING;
}

function holds(role: Role): (view: PlayerView) => boolean {
  return (view) => view.role?.role === role;
}

/**
 * What this phone shows at Night if its Player is asleep: every living
 * Player but those acting, and those too while the Narrator announces their
 * Turn. `null` when the phone is awake: outside the Night, during its
 * Player's own Turn, or for a Spectator, who sees it all.
 */
export function sleepingScreen(view: PlayerView): SleepingScreen | null {
  const { moment } = view;
  if (!moment || view.spectating) return null;
  const acting = ACTING[moment.type];
  if (!acting) return null;
  const alive = view.players.find((p) => p.id === view.you)?.alive ?? false;
  if (alive && acting(view) && view.announcing === null) return null;
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
export function pausedBy(view: PlayerView): string | null {
  return view.paused ? (view.players.find((p) => p.id === view.host)?.name ?? "?") : null;
}
