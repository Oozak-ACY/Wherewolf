import { useEffect, useState } from "react";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCounts } from "./generated/RoleCounts";
import type { Settings } from "./generated/Settings";
import type { Timers } from "./generated/Timers";
import { ROLES, roleTotal } from "./lobbyRoom";
import { cardImageUrl } from "./RoleCardView";
import {
  fr,
  MAX_CUPIDS,
  MAX_PLAYERS,
  MAX_THIEVES,
  MAX_WITCHES,
  TIMER_MAX_SECONDS,
  TIMER_MIN_SECONDS,
} from "./strings";

/** The most of each Role a Game may hold, when there is a limit. */
const MAX_COUNTS: Partial<Record<Role, number>> = { witch: MAX_WITCHES, cupid: MAX_CUPIDS, thief: MAX_THIEVES };
const TIMERS = Object.keys(fr.settings.timer) as (keyof Timers)[];

type Props = {
  view: PlayerView;
  onChange: (settings: Settings) => void;
};

/** The Roles in play as a sideways row of cards; only the Host gets the steppers. */
export function RoleCards({ view, onChange }: Props) {
  const isHost = view.you === view.host;
  const { settings } = view;
  const total = roleTotal(settings.roles);
  const followsSuggestion = ROLES.every((r) => settings.roles[r] === view.suggestedRoles[r]);
  const setRoles = (roles: RoleCounts) => onChange({ ...settings, roles });

  return (
    <section className="stack">
      <div className="section-head">
        <h2>{fr.settings.roles}</h2>
        <span className="role-total" data-match={total === view.players.length}>
          {fr.settings.rolesCount(total, view.players.length)}
        </span>
      </div>
      <ul className="role-row">
        {ROLES.map((role) => {
          const name = fr.roles[role].name;
          const count = settings.roles[role];
          return (
            <li key={role}>
              <div className="role-art">
                <img className={count ? "card-art" : "card-art off"} src={cardImageUrl(role)} alt="" />
                <span className="role-count">{count}</span>
              </div>
              <span className="role-name">{name}</span>
              {isHost && (
                <span className="stepper">
                  <button
                    aria-label={fr.settings.less(name)}
                    disabled={count === 0}
                    onClick={() => setRoles({ ...settings.roles, [role]: count - 1 })}
                  >
                    −
                  </button>
                  <button
                    aria-label={fr.settings.more(name)}
                    disabled={total >= MAX_PLAYERS || count >= (MAX_COUNTS[role] ?? Infinity)}
                    onClick={() => setRoles({ ...settings.roles, [role]: count + 1 })}
                  >
                    +
                  </button>
                </span>
              )}
            </li>
          );
        })}
      </ul>
      {isHost && !followsSuggestion && (
        <button className="quiet" onClick={() => setRoles(view.suggestedRoles)}>
          {fr.settings.suggested}
        </button>
      )}
    </section>
  );
}

/** The timers, Hidden Roles and the Grace period, folded under one row; editable by the Host only. */
export function MoreSettings({ view, onChange }: Props) {
  const isHost = view.you === view.host;
  const { settings } = view;
  const setTimer = (timer: keyof Timers, seconds: number) =>
    onChange({ ...settings, timers: { ...settings.timers, [timer]: seconds } });

  return (
    <details className="more-settings">
      <summary>
        <span>{fr.settings.disclosure}</span>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
          <path d="M9 6l6 6-6 6" />
        </svg>
      </summary>
      <div className="stack">
        <label className="toggle">
          <span>{fr.settings.hiddenRoles}</span>
          <input
            type="checkbox"
            checked={settings.hiddenRoles}
            disabled={!isHost}
            onChange={(e) => onChange({ ...settings, hiddenRoles: e.target.checked })}
          />
        </label>
        <p className="muted">{settings.hiddenRoles ? fr.settings.hiddenRolesOn : fr.settings.hiddenRolesOff}</p>

        <ul className="timers">
          <li>
            <label htmlFor="grace-period">{fr.settings.gracePeriod}</label>
            <Seconds
              id="grace-period"
              editable={isHost}
              seconds={settings.gracePeriod}
              onCommit={(gracePeriod) => onChange({ ...settings, gracePeriod })}
            />
          </li>
        </ul>
        <p className="muted">{fr.settings.gracePeriodHint}</p>

        <ul className="timers">
          {TIMERS.map((timer) => (
            <li key={timer}>
              <label htmlFor={`timer-${timer}`}>{fr.settings.timer[timer]}</label>
              <Seconds
                id={`timer-${timer}`}
                editable={isHost}
                seconds={settings.timers[timer]}
                onCommit={(seconds) => setTimer(timer, seconds)}
              />
            </li>
          ))}
        </ul>
      </div>
    </details>
  );
}

/** The Host's start button and why it is blocked, or the waiting line for everyone else. */
export function StartFooter({ view, onStart }: { view: PlayerView; onStart: () => void }) {
  if (view.you !== view.host) {
    return (
      <footer className="room-footer">
        <p className="waiting">{fr.settings.waitingForHost}</p>
      </footer>
    );
  }
  return (
    <footer className="room-footer">
      <button className="primary" disabled={view.startBlockedBy !== null} onClick={onStart}>
        {fr.settings.start}
      </button>
      {view.startBlockedBy && (
        <p className={view.startBlockedBy === "playersDisconnected" ? "error" : "muted"}>
          {fr.rejection[view.startBlockedBy]}
        </p>
      )}
    </footer>
  );
}

function Seconds(props: { id: string; editable: boolean; seconds: number; onCommit: (seconds: number) => void }) {
  if (props.editable) return <TimerInput id={props.id} seconds={props.seconds} onCommit={props.onCommit} />;
  return (
    <strong id={props.id}>
      {props.seconds} {fr.settings.seconds}
    </strong>
  );
}

/** A number of seconds, sent once the Host is done typing it. */
function TimerInput(props: { id: string; seconds: number; onCommit: (seconds: number) => void }) {
  const [draft, setDraft] = useState(String(props.seconds));
  useEffect(() => setDraft(String(props.seconds)), [props.seconds]);

  const commit = () => {
    const seconds = Math.round(Number(draft));
    // An empty or out-of-bounds value would be refused and left on screen: undo it.
    const valid = draft.trim() !== "" && seconds >= TIMER_MIN_SECONDS && seconds <= TIMER_MAX_SECONDS;
    if (valid && seconds !== props.seconds) props.onCommit(seconds);
    else setDraft(String(props.seconds));
  };

  return (
    <span className="timer-input">
      <input
        id={props.id}
        type="number"
        inputMode="numeric"
        min={TIMER_MIN_SECONDS}
        max={TIMER_MAX_SECONDS}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
      />
      {fr.settings.seconds}
    </span>
  );
}
