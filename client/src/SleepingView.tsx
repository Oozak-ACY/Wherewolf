import { Disconnected, Paused } from "./Banners";
import { CardBack } from "./PlayerGrid";
import type { SleepingScreen } from "./sleepingScreen";
import { fr } from "./strings";

/**
 * A sleeping phone at Night under the Narrator bar silent and the same for
 * every Role
 * It shows nothing but the screen and this phone's own connection in the
 * layout of an acting phone with a card back for the Role
 * Outside the Night with `night` off the same screen waits for the Narrator
 * on parchment without the card or the call to sleep
 */
export function SleepingView(props: { screen: SleepingScreen; night: boolean; reconnecting: boolean }) {
  const { screen } = props;
  return (
    <section className="sleeping">
      {props.reconnecting && <Disconnected />}
      {screen.pausedBy !== null && <Paused hostName={screen.pausedBy} />}
      {props.night ? (
        <header className="turn-head">
          <CardBack />
          <div className="turn-text">
            <h1>{fr.game.sleepingTitle}</h1>
            <p>{fr.game.asleep}</p>
          </div>
        </header>
      ) : (
        <div className="sleeping-body">
          <p className="narrator" aria-live="polite">{screen.line}</p>
        </div>
      )}
    </section>
  );
}
