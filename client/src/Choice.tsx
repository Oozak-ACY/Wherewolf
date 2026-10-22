import { useState, type ReactNode } from "react";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { Picking } from "./ground";
import { initial } from "./lobby";
import { fr } from "./strings";

/** A footer action besides the Players such as passing that is selected then confirmed like a Player */
export type Action = { label: string; confirmLabel?: string; quiet?: boolean; onChoose: () => void };

type Selection = { player: PlayerId } | { action: string };

/**
 * A choice of one Player among `players` then the screen's footer with
 * `actions` and `children` as its other actions
 * In `tap` mode each tap sends the choice and `chosen` is the one sent
 * In `confirm` mode a tap on a row or an action only selects it and the
 * footer's confirm button named by `confirmLabel` sends it
 */
export function Choice(props: {
  mode: Picking;
  players: PlayerSummary[];
  chosen?: PlayerId | null;
  detail?: (id: PlayerId) => string | null;
  count?: (id: PlayerId) => number;
  mark?: ReactNode;
  confirmLabel?: (name: string) => string;
  onChoose: (id: PlayerId) => void;
  actions?: Action[];
  children?: ReactNode;
}) {
  const [selected, setSelected] = useState<Selection | null>(null);
  const confirming = props.mode === "confirm";
  const actions = props.actions ?? [];
  // A selected Player who died meanwhile is no longer a choice
  const picked = selected && "player" in selected ? props.players.find((p) => p.id === selected.player) : undefined;
  // An action gone meanwhile such as a used potion is no longer a choice
  const action = selected && "action" in selected ? actions.find((a) => a.label === selected.action) : undefined;
  const pressed = confirming ? picked?.id : props.chosen;
  const confirm = !confirming
    ? null
    : picked
      ? { label: props.confirmLabel?.(picked.name) ?? picked.name, send: () => props.onChoose(picked.id) }
      : action
        ? { label: action.confirmLabel ?? fr.game.confirm.action, send: action.onChoose }
        : null;
  return (
    <>
      {props.players.length > 0 && (
        <PickRows
          players={props.players}
          pressed={(id) => id === pressed}
          detail={props.detail}
          count={props.count}
          mark={props.mark}
          onTap={confirming ? (id) => setSelected({ player: id }) : props.onChoose}
        />
      )}
      {(confirm || actions.length > 0 || props.children) && (
        <Footer>
          {confirm && (
            <button className="primary" onClick={confirm.send}>
              {confirm.label}
            </button>
          )}
          {actions.map((a) => {
            const chosen = action === a;
            return (
              <button
                key={a.label}
                className={[a.quiet === false ? null : "quiet", chosen && "chosen"].filter(Boolean).join(" ")}
                aria-pressed={confirming ? chosen : undefined}
                onClick={confirming ? () => setSelected({ action: a.label }) : a.onChoose}
              >
                {a.label}
              </button>
            );
          })}
          {props.children}
        </Footer>
      )}
    </>
  );
}

/**
 * One full-width pick row per Player with who picked them and how many and
 * `mark` once pressed
 * Read only without `onTap` for those watching a choice
 */
export function PickRows(props: {
  players: PlayerSummary[];
  pressed?: (id: PlayerId) => boolean;
  detail?: (id: PlayerId) => string | null;
  count?: (id: PlayerId) => number;
  mark?: ReactNode;
  onTap?: (id: PlayerId) => void;
}) {
  return (
    <ul className="choices">
      {props.players.map((p) => {
        const detail = props.detail?.(p.id);
        const count = props.count?.(p.id);
        const pressed = props.pressed?.(p.id) ?? false;
        const { onTap } = props;
        const row = (
          <>
            <span className="avatar" aria-hidden="true">
              {initial(p.name)}
            </span>
            <span className="pick-name">
              <span>{p.name}</span>
              {detail && <span className="muted">{detail}</span>}
            </span>
            {pressed && props.mark}
            {count !== undefined && (
              <span className="pick-count" data-zero={count === 0}>
                {count}
              </span>
            )}
          </>
        );
        return (
          <li key={p.id}>
            {onTap ? (
              <button className="pick-row" aria-pressed={pressed} onClick={() => onTap(p.id)}>
                {row}
              </button>
            ) : (
              <span className="pick-row">{row}</span>
            )}
          </li>
        );
      })}
    </ul>
  );
}

/** The screen's main actions stuck to the bottom within thumb reach */
export function Footer(props: { children: ReactNode }) {
  return <div className="footer">{props.children}</div>;
}
