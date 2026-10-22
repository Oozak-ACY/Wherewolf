import type { Ballot } from "./generated/Ballot";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { fr } from "./strings";

/** Who the Player is to the Game, under their name in the Day's Player list. */
export function listTag(view: PlayerView, player: PlayerSummary): string {
  const parts: string[] = [];
  if (player.id === view.you) parts.push(fr.lobby.you);
  if (player.revealedRole) parts.push(fr.roles[player.revealedRole].name);
  if (!player.alive) parts.push(fr.game.spectator);
  if (!player.connected) parts.push(fr.lobby.offline);
  return parts.join(" · ");
}

/** Who voted for `designated`, in the order they voted. */
export function votersFor(ballots: Ballot[], designated: PlayerId): PlayerId[] {
  return ballots.filter((b) => b.designated === designated).map((b) => b.voter);
}

/** Who abstained. */
export function abstainers(ballots: Ballot[]): PlayerId[] {
  return ballots.filter((b) => b.designated === null).map((b) => b.voter);
}

/** The names of `ids`, this Player's own marked as theirs. `null` for nobody. */
export function nameList(view: PlayerView, ids: PlayerId[]): string | null {
  if (ids.length === 0) return null;
  return ids.map((id) => (id === view.you ? `${nameOf(view, id)} (${fr.lobby.you})` : nameOf(view, id))).join(", ");
}

/** This Player's own vote in the Election or the Vote so far. `null` before they vote. */
export function yourVote(view: PlayerView): string | null {
  const { moment } = view;
  if (moment?.type !== "election" && moment?.type !== "vote") return null;
  const ballot = moment.yourBallot;
  if (!ballot) return null;
  if (ballot.designated === null) return fr.game.youAbstained;
  return fr.game.yourVote(nameOf(view, ballot.designated));
}

/** How a tied Vote is settled, with the Mayor named. `null` without a Mayor. */
export function mayorNote(view: PlayerView): string | null {
  if (view.mayor === null) return null;
  return view.mayor === view.you ? fr.game.mayorTieYou : fr.game.mayorTie(nameOf(view, view.mayor));
}

/** The name of Player `id`. */
export function nameOf(view: PlayerView, id: PlayerId): string {
  return view.players.find((p) => p.id === id)?.name ?? "?";
}
