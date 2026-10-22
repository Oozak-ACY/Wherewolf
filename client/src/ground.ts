import type { PlayerView } from "./generated/PlayerView";
import { isNightTurn, sleepingScreen, waitingScreen } from "./sleepingScreen";

/**
 * The background a screen sits on with one per mood
 * Parchment for public screens and the Day and night for the Night and
 * reveal for the private Role card
 */
export type Ground = "parchment" | "night" | "reveal";

/**
 * The ground of the screen this phone shows with `cardOpen` telling whether
 * the Player has their Role card open
 * A sleeping or waiting phone hides the card
 */
export function ground(view: PlayerView, cardOpen: boolean): Ground {
  if (sleepingScreen(view)) return "night";
  const card = (cardOpen && view.role !== null) || view.moment?.type === "roleReveal";
  if (card && !waitingScreen(view)) return "reveal";
  return view.moment && isNightTurn(view.moment) ? "night" : "parchment";
}

const THEME_COLORS: Record<Ground, string> = {
  parchment: "#1F3F6E",
  night: "#0A1220",
  reveal: "#1A1210",
};

/** The browser's `theme-color` on this ground */
export function themeColor(ground: Ground): string {
  return THEME_COLORS[ground];
}

/** How pick rows send a choice either on each tap or once a confirm step follows the selection */
export type Picking = "tap" | "confirm";

/**
 * How this Player's pick rows send their choice with a confirm step for the
 * irreversible choices and a tap for those that stay changeable
 * `null` when this Player has no Player to pick
 */
export function picking(view: PlayerView): Picking | null {
  const { moment } = view;
  if (!moment || view.announcing !== null || view.spectating) return null;
  const alive = view.players.find((p) => p.id === view.you)?.alive ?? false;
  const role = view.role?.role;
  switch (moment.type) {
    case "cupidsTurn":
      return alive && role === "cupid" ? "confirm" : null;
    case "thiefsTurn":
      return alive && role === "thief" ? "confirm" : null;
    case "seersTurn":
      return alive && role === "seer" && !moment.inspection && !moment.passed ? "confirm" : null;
    case "witchsTurn":
      return alive && view.role?.potions?.poison && moment.witch && !moment.witch.passed ? "confirm" : null;
    case "werewolvesTurn":
      return alive && moment.picks ? "tap" : null;
    case "election":
    case "vote":
      return alive ? "tap" : null;
    case "tieBreak":
      return alive && view.mayor === view.you ? "confirm" : null;
    case "huntersShot":
      return moment.hunter === view.you ? "confirm" : null;
    case "succession":
      return moment.mayor === view.you ? "confirm" : null;
    default:
      return null;
  }
}
