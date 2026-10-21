import type { PlayerView } from "./generated/PlayerView";
import { isNightTurn, sleepingScreen, waitingScreen } from "./sleepingScreen";

/**
 * The background a screen sits on, one per mood: parchment for public
 * screens and the Day, night for the Night, reveal for the private Role card.
 */
export type Ground = "parchment" | "night" | "reveal";

/**
 * The ground of the screen this phone shows, `cardOpen` being whether the
 * Player has their Role card open. A sleeping or waiting phone hides the card.
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

/** The browser's `theme-color` on this ground. */
export function themeColor(ground: Ground): string {
  return THEME_COLORS[ground];
}
