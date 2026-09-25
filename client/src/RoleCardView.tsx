import { useState } from "react";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCard } from "./generated/RoleCard";
import { fr } from "./strings";

/**
 * The artwork for a Role, by a fixed convention: `client/public/roles/<role>.png`
 * (e.g. `werewolf.png`). Dropping in a file replaces the placeholder, no code change.
 */
export function roleImageUrl(role: Role): string {
  return `/roles/${role}.png`;
}

/** A Player's own Role card, shown privately on their phone. */
export function RoleCardView(props: { card: RoleCard; view: PlayerView; onClose: () => void }) {
  const { card, view } = props;
  const role = fr.roles[card.role];
  const camp = fr.camps[card.camp];
  const fellows = card.fellowWerewolves.map(
    (id) => view.players.find((p) => p.id === id)?.name ?? "?",
  );

  return (
    <div className="overlay" role="dialog" aria-modal="true" aria-labelledby="role-name">
      <article className={`role-card camp-${card.camp}`}>
        <p className="muted">{fr.roleCard.you}</p>
        <h1 id="role-name">{role.name}</h1>
        <RoleArt role={card.role} />
        <p>{role.power}</p>
        <dl>
          <dt>{fr.roleCard.camp}</dt>
          <dd>{camp.name}</dd>
          <dt>{fr.roleCard.goal}</dt>
          <dd>{camp.goal}</dd>
        </dl>
        {card.role === "werewolf" && (
          <p className="fellows">
            {fellows.length > 0
              ? `${fr.roleCard.fellowWerewolves} : ${fellows.join(", ")}`
              : fr.roleCard.loneWerewolf}
          </p>
        )}
        <button className="primary" onClick={props.onClose} autoFocus>
          {fr.roleCard.close}
        </button>
      </article>
    </div>
  );
}

/** The Role's artwork, or a placeholder until the image file exists. */
function RoleArt({ role }: { role: Role }) {
  const [missing, setMissing] = useState(false);
  if (missing) {
    return (
      <div className="role-art placeholder" aria-hidden="true">
        {fr.roles[role].name.charAt(0)}
      </div>
    );
  }
  return (
    <img className="role-art" src={roleImageUrl(role)} alt="" onError={() => setMissing(true)} />
  );
}
