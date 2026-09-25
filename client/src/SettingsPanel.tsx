import { useEffect, useState } from "react";
import type { PlayerView } from "./generated/PlayerView";
import type { Role } from "./generated/Role";
import type { RoleCounts } from "./generated/RoleCounts";
import type { Settings } from "./generated/Settings";
import type { Timers } from "./generated/Timers";
import { fr, MAX_PLAYERS, TIMER_MAX_SECONDS, TIMER_MIN_SECONDS } from "./strings";

// The order the Night calls them in, then the Villagers.
const ROLES: Role[] = ["werewolf", "seer", "witch", "hunter", "villager"];
const TIMERS = Object.keys(fr.settings.timer) as (keyof Timers)[];

type Props = {
  view: PlayerView;
  onChange: (settings: Settings) => void;
  onStart: () => void;
};

/** The Settings every Player sees; only the Host can edit them and start. */
export function SettingsPanel({ view, onChange, onStart }: Props) {
  const isHost = view.you === view.host;
  const { settings } = view;
  const roleTotal = ROLES.reduce((sum, role) => sum + settings.roles[role], 0);
  const followsSuggestion = ROLES.every((r) => settings.roles[r] === view.suggestedRoles[r]);

  const setRoles = (roles: RoleCounts) => onChange({ ...settings, roles });
  const setTimer = (timer: keyof Timers, seconds: number) =>
    onChange({ ...settings, timers: { ...settings.timers, [timer]: seconds } });

  return (
    <section className="card stack settings">
      <h2>{fr.settings.title}</h2>

      <h3>{fr.settings.roles}</h3>
      <ul className="role-counts">
        {ROLES.map((role) => {
          const name = fr.roles[role].name;
          const count = settings.roles[role];
          return (
            <li key={role}>
              <span>{name}</span>
              {isHost ? (
                <span className="stepper">
                  <button
                    aria-label={fr.settings.less(name)}
                    disabled={count === 0}
                    onClick={() => setRoles({ ...settings.roles, [role]: count - 1 })}
                  >
                    −
                  </button>
                  <strong>{count}</strong>
                  <button
                    aria-label={fr.settings.more(name)}
                    disabled={roleTotal >= MAX_PLAYERS}
                    onClick={() => setRoles({ ...settings.roles, [role]: count + 1 })}
                  >
                    +
                  </button>
                </span>
              ) : (
                <strong>{count}</strong>
              )}
            </li>
          );
        })}
      </ul>
      <p className={view.startBlockedBy === "roleCountMismatch" ? "error" : "muted"}>
        {fr.settings.rolesCount(roleTotal, view.players.length)}
      </p>
      {isHost && !followsSuggestion && (
        <button onClick={() => setRoles(view.suggestedRoles)}>{fr.settings.suggested}</button>
      )}

      <details>
        <summary>
          <h3>{fr.settings.timers}</h3>
        </summary>
        <ul className="timers">
          {TIMERS.map((timer) => (
            <li key={timer}>
              <label htmlFor={`timer-${timer}`}>{fr.settings.timer[timer]}</label>
              {isHost ? (
                <TimerInput
                  id={`timer-${timer}`}
                  seconds={settings.timers[timer]}
                  onCommit={(seconds) => setTimer(timer, seconds)}
                />
              ) : (
                <strong id={`timer-${timer}`}>
                  {settings.timers[timer]} {fr.settings.seconds}
                </strong>
              )}
            </li>
          ))}
        </ul>
      </details>

      {isHost ? (
        <>
          <button className="primary" disabled={view.startBlockedBy !== null} onClick={onStart}>
            {fr.settings.start}
          </button>
          {view.startBlockedBy && <p className="muted">{fr.rejection[view.startBlockedBy]}</p>}
        </>
      ) : (
        <p className="muted">{fr.settings.waitingForHost}</p>
      )}
    </section>
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
