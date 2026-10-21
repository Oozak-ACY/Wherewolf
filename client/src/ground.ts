import type { PlayerView } from "./generated/PlayerView";
import { sleepingScreen, waitingScreen } from "./sleepingScreen";

/**
 * The background a screen sits on, one per mood: parchment for public and
 * daytime screens, night for the Night, reveal for the private Role card.
 */
export type Ground = "parchment" | "night" | "reveal";

const NIGHT_TURNS = new Set(["cupidsTurn", "loversTurn", "thiefsTurn", "seersTurn", "werewolvesTurn", "witchsTurn"]);

/**
 * The ground of the screen this phone shows, `cardOpen` being whether the
 * Player has their Role card open. A sleeping or waiting phone hides the card.
 */
export function ground(view: PlayerView, cardOpen: boolean): Ground {
  const night = view.moment !== null && NIGHT_TURNS.has(view.moment.type);
  if (sleepingScreen(view)) return "night";
  if (waitingScreen(view)) return night ? "night" : "parchment";
  if ((cardOpen && view.role) || view.moment?.type === "roleReveal") return "reveal";
  return night ? "night" : "parchment";
}

const THEME_COLORS: Record<Ground, string> = {
  parchment: "#1F3F6E",
  night: "#0A1220",
  reveal: "#1A1210",
};

/** The browser's `theme-color` on this ground. */
export function themeColor(ground: Ground): string {
  return THEME_COLORS[ground];
}
