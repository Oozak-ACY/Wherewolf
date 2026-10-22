import type { Moment } from "./generated/Moment";
import type { PlayerView } from "./generated/PlayerView";
import type { CardArt } from "./RoleCardView";
import { isNightTurn, sleepingScreen, waitingScreen } from "./sleepingScreen";
import { fr } from "./strings";

/** What tops every Night Turn's screen with the card of who acts and its title */
export type NightTurn = { card: CardArt; title: string };

const TURNS: Partial<Record<Moment["type"], NightTurn>> = {
  cupidsTurn: { card: "cupid", title: fr.roles.cupid.name },
  loversTurn: { card: "cupid", title: fr.camps.lovers.name },
  thiefsTurn: { card: "thief", title: fr.roles.thief.name },
  seersTurn: { card: "seer", title: fr.roles.seer.name },
  werewolvesTurn: { card: "werewolf", title: fr.camps.werewolves.name },
  witchsTurn: { card: "witch", title: fr.roles.witch.name },
};

/**
 * The Night Turn this phone shows awake to the acting Player or a Spectator
 * `null` outside the Night and on a sleeping or waiting phone
 */
export function nightTurn(view: PlayerView): NightTurn | null {
  const { moment } = view;
  if (!moment || !isNightTurn(moment) || sleepingScreen(view) || waitingScreen(view)) return null;
  return TURNS[moment.type] ?? null;
}
