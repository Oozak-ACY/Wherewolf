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
 * Whether this Player's Role card opens by itself face down, as the card
 * they hold goes from `before` to `now`: when the Roles are dealt, and when
 * the Theft changes it, for the Thief and for the Player they robbed.
 */
export function cardOpensByItself(before: Role | null, now: Role | null): boolean {
  return now !== null && now !== before;
}

/** Face down when the card opened by itself, face up once revealed or when the Player asked for it */
export type Face = "down" | "up";

/** What the card's one button does */
export type CardAction = "reveal" | "confirm" | "close";

/** The card's button: reveal it face down, then confirm the Role during the Role reveal or close it */
export function cardAction(face: Face, view: PlayerView): CardAction {
  if (face === "down") return "reveal";
  const confirming = view.moment?.type === "roleReveal" && !view.moment.confirmed && !view.spectating;
  return confirming ? "confirm" : "close";
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
 * face down until they reveal it
 * During the Role reveal it closes only once the Player confirms their Role
 */
export function RoleCardView(props: {
  card: RoleCard;
  view: PlayerView;
  face: Face;
  onReveal: () => void;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const { card, view, face } = props;
  const dialog = useRef<HTMLDialogElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const action = cardAction(face, view);
  const confirming = cardAction("up", view) === "confirm";
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
      <article className="role-card" aria-live="polite">
        {face === "down" ? (
          <>
            <h1 id="role-name" className="eyebrow">
              {fr.game.myCard}
            </h1>
            <CardBack />
          </>
        ) : (
          <>
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
          </>
        )}
        <button
          key="action"
          ref={button}
          className="primary"
          onClick={() => {
            if (action === "reveal") return props.onReveal();
            if (action === "confirm") props.onConfirm();
            props.onClose();
          }}
        >
          {action === "reveal" ? fr.game.revealRole : action === "confirm" ? fr.game.confirmRole : fr.roleCard.close}
        </button>
      </article>
    </dialog>
  );
}

/** The back of every card which tells nothing about the Role */
function CardBack() {
  return (
    <svg className="card-art reveal card-back" viewBox="0 0 18 32" aria-hidden="true">
      <rect x="1" y="1" width="16" height="30" rx="1.5" fill="var(--ww-night)" stroke="var(--ww-gold)" strokeWidth="0.5" />
      <rect x="2.5" y="2.5" width="13" height="27" rx="1" fill="none" stroke="var(--ww-gold)" strokeWidth="0.25" />
      <path d="M10.5 11.5a4.5 4.5 0 1 0 2.9 7 5.3 5.3 0 0 1-2.9-7z" fill="var(--ww-gold-light)" />
    </svg>
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
