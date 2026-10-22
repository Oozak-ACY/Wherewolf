import { useState, type ReactNode } from "react";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { Picking } from "./ground";
import { initial } from "./lobby";

/**
 * A choice of one Player among `players`, then the screen's footer with
 * `children` as its other actions. In `tap` mode each tap sends the choice,
 * `chosen` being the one sent. In `confirm` mode a tap only selects, and the
 * footer's confirm button, named by `confirmLabel`, sends it.
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
  children?: ReactNode;
}) {
  const [selected, setSelected] = useState<PlayerId | null>(null);
  const confirming = props.mode === "confirm";
  // A selected Player who died meanwhile is no longer a choice
  const picked = props.players.find((p) => p.id === selected);
  const pressed = confirming ? picked?.id : props.chosen;
  return (
    <>
      <PickRows
        players={props.players}
        pressed={(id) => id === pressed}
        detail={props.detail}
        count={props.count}
        mark={props.mark}
        onTap={confirming ? setSelected : props.onChoose}
      />
      {((confirming && picked) || props.children) && (
        <Footer>
          {confirming && picked && (
            <button className="primary" onClick={() => props.onChoose(picked.id)}>
              {props.confirmLabel?.(picked.name) ?? picked.name}
            </button>
          )}
          {props.children}
        </Footer>
      )}
    </>
  );
}

/**
 * One full-width pick row per Player, with who picked them and how many, and
 * `mark` once pressed. Read only without `onTap`, for those watching a choice.
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

/** The screen's main actions, stuck to the bottom within thumb reach. */
export function Footer(props: { children: ReactNode }) {
  return <div className="footer">{props.children}</div>;
}
