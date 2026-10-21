import { ConnectionLost, Paused } from "./Banners";
import type { SleepingScreen } from "./sleepingScreen";
import { fr } from "./strings";

/**
 * A sleeping phone at Night, under the Narrator bar: silent, and the same
 * for every Role. It shows nothing but the screen and this phone's own
 * connection. Outside the Night (`night` off), the same screen waits for
 * the Narrator on parchment, without the moon or the call to sleep.
 */
export function SleepingView(props: { screen: SleepingScreen; night: boolean; reconnecting: boolean }) {
  const { screen } = props;
  return (
    // No aria-live: a screen reader would speak the line out loud at the table.
    <section className="sleeping">
      {props.reconnecting && <ConnectionLost />}
      {screen.pausedBy !== null && <Paused host={screen.pausedBy} />}
      <div className="sleeping-body">
        {props.night ? (
          <>
            <svg viewBox="0 0 40 40" className="moon" aria-hidden="true">
              <path d="M24 9a11 11 0 1 0 7 17A13 13 0 0 1 24 9z" fill="currentColor" />
            </svg>
            <p>{fr.game.asleep}</p>
          </>
        ) : (
          <p className="narrator">{screen.line}</p>
        )}
      </div>
    </section>
  );
}
