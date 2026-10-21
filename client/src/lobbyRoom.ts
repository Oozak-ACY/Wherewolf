import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCounts } from "./generated/RoleCounts";
import { fr } from "./strings";

/** The order the Night calls them in, then the Villagers. */
export const ROLES: Role[] = ["werewolf", "cupid", "thief", "seer", "witch", "hunter", "villager"];

export function roleTotal(roles: RoleCounts): number {
  return ROLES.reduce((sum, role) => sum + roles[role], 0);
}

/** Who the Player is to this Lobby, under their name on their tile. */
export function playerTag(view: PlayerView, player: PlayerSummary): string {
  const parts: string[] = [];
  if (player.id === view.host) parts.push(fr.lobby.host);
  if (player.id === view.you) parts.push(fr.lobby.you);
  if (!player.connected) parts.push(fr.lobby.offline);
  return parts.length ? parts.join(" · ") : fr.lobby.ready;
}

/** The first character of a name, an emoji kept whole. */
export function initial(name: string): string {
  return (Array.from(name.trim())[0] ?? "").toUpperCase();
}
