import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { fr } from "./strings";

/** Fewer, bigger tiles while the Lobby fills up; 12 still fit a portrait phone. */
export function gridSize(count: number): "small" | "medium" | "large" {
  return count <= 4 ? "small" : count <= 9 ? "medium" : "large";
}

/** One tile per seated Player, in the order they joined. */
export function PlayerGrid({ view }: { view: PlayerView }) {
  return (
    <ul className="call-grid" data-size={gridSize(view.players.length)}>
      {view.players.map((player) => (
        <li key={player.id} className={tileClasses(player).join(" ")}>
          <CardBack />
          <TileLabel view={view} player={player} />
        </li>
      ))}
    </ul>
  );
}

/** How a tile looks for who the Player is now: offline, eliminated. */
export function tileClasses(player: PlayerSummary): string[] {
  const classes = ["tile"];
  if (!player.connected) classes.push("offline");
  if (!player.alive) classes.push("dead");
  return classes;
}

/** The Player's name, with who they are to the Game. */
export function TileLabel({ view, player }: { view: PlayerView; player: PlayerSummary }) {
  return (
    <div className="tile-label">
      <span className="name">
        {player.name}
        {player.id === view.you && <span className="muted"> ({fr.lobby.you})</span>}
      </span>
      {player.id === view.host && <span className="badge">{fr.lobby.host}</span>}
      {player.id === view.mayor && <span className="badge">{fr.lobby.mayor}</span>}
      {player.revealedRole && <span className="badge role">{fr.roles[player.revealedRole].name}</span>}
      {!player.alive && !player.revealedRole && <span className="muted">{fr.game.spectator}</span>}
      {!player.connected && <span className="muted">{fr.lobby.offline}</span>}
    </div>
  );
}

/** A Player without a camera shows as the back of a Role card. */
export function CardBack() {
  return (
    <div className="card-back" aria-hidden="true">
      <svg viewBox="0 0 40 40" className="emblem">
        <circle cx="20" cy="20" r="17" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M24 9a11 11 0 1 0 7 17A13 13 0 0 1 24 9z" fill="currentColor" />
      </svg>
    </div>
  );
}
