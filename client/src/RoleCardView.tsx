import { useEffect, useRef, useState } from "react";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCard } from "./generated/RoleCard";
import { fr } from "./strings";

/** A Role or the Mayor title painted as a 9:16 stained glass card */
export type CardArt = Role | "mayor";

/** The artwork served from `client/public/roles/<card>.webp` */
export function cardImageUrl(card: CardArt): string {
  return `/roles/${card}.webp`;
}

/**
 * This Player's partner, if they are one of the Lovers. The bond is between
 * the two Players, whatever their Roles.
 */
export function partnerOf(view: PlayerView): PlayerId | undefined {
  return view.lovers.includes(view.you) ? view.lovers.find((id) => id !== view.you) : undefined;
}

/**
 * Whether this Player's Role card turns face up by itself, as the card they
 * hold goes from `before` to `now`: when the Roles are dealt, and when the
 * Theft changes it, for the Thief and for the Player they robbed.
 */
export function cardTurnsFaceUp(before: Role | null, now: Role | null): boolean {
  return now !== null && now !== before;
}

/** One box of allies on the reveal untitled for a lone Werewolf */
export type AlliesBox = { title: string | null; names: string };

/** Who this Player knows is on their side with the other Werewolves then their partner */
export function allies(card: RoleCard, view: PlayerView): AlliesBox[] {
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const boxes: AlliesBox[] = [];
  if (card.role === "werewolf") {
    const pack = card.fellowWerewolves.map(name);
    boxes.push(
      pack.length > 0
        ? { title: fr.roleCard.fellowWerewolves, names: pack.join(", ") }
        : { title: null, names: fr.roleCard.loneWerewolf },
    );
  }
  const partner = partnerOf(view);
  if (partner !== undefined) boxes.push({ title: fr.roleCard.partner, names: name(partner) });
  return boxes;
}

/**
 * A Player's own Role card shown privately on their phone in a modal dialog
 * During the Role reveal closing it the first time confirms the Player has
 * seen their Role
 */
export function RoleCardView(props: {
  card: RoleCard;
  view: PlayerView;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const { card, view } = props;
  const dialog = useRef<HTMLDialogElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const confirming =
    view.moment?.type === "roleReveal" && !view.moment.confirmed && !view.spectating;
  const role = fr.roles[card.role];

  useEffect(() => {
    dialog.current?.showModal();
    button.current?.focus();
  }, []);

  return (
    <dialog
      ref={dialog}
      className="reveal"
      aria-labelledby="role-name"
      // The Role reveal ends only on the confirm button even when Escape cannot be prevented
      onCancel={(e) => confirming && e.preventDefault()}
      onClose={() => (confirming && dialog.current?.isConnected ? dialog.current.showModal() : props.onClose())}
    >
      <article className="role-card">
        <p className="eyebrow">{fr.roleCard.you}</p>
        <RoleArt role={card.role} />
        <h1 id="role-name">{role.name}</h1>
        <p className="camp">
          <span className="eyebrow">{fr.roleCard.camp}</span>
          <span className="camp-pill">{fr.camps[card.camp].name}</span>
        </p>
        <p className="power">{role.power}</p>
        <p className="goal">
          <span className="eyebrow">{fr.roleCard.goal}</span>
          <span>{fr.camps[card.camp].goal}</span>
        </p>
        {allies(card, view).map((box) => (
          <p key={box.names} className="allies">
            {box.title && <span className="eyebrow">{box.title}</span>}
            <span>{box.names}</span>
          </p>
        ))}
        <button
          ref={button}
          className="primary"
          onClick={() => {
            if (confirming) props.onConfirm();
            props.onClose();
          }}
        >
          {confirming ? fr.game.confirmRole : fr.roleCard.close}
        </button>
      </article>
    </dialog>
  );
}

/** The Role's artwork, or a placeholder until the image file exists. */
function RoleArt({ role }: { role: Role }) {
  const [missing, setMissing] = useState(false);
  if (missing) {
    return (
      <div className="card-art reveal placeholder" aria-hidden="true">
        {fr.roles[role].name.charAt(0)}
      </div>
    );
  }
  return (
    <img className="card-art reveal" src={cardImageUrl(role)} alt="" onError={() => setMissing(true)} />
  );
}
