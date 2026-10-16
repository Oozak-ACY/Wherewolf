import { useState } from "react";
import { BUSES, MAX_LEVEL, type MixerLevels } from "./mixer";
import { fr } from "./strings";
import { playTableTest, setTableLevels, tableLevels } from "./tableAudio";

/**
 * The Host's discreet mixer during the Game: one fader per bus of the Table
 * device's mix, 0 to 150%, and a short test sound. Levels apply at once and
 * stay on this phone for the next Games.
 */
export function MixerControl() {
  const [open, setOpen] = useState(false);
  const [levels, setLevels] = useState(tableLevels);

  const change = (bus: keyof MixerLevels, percent: number) => {
    setLevels(setTableLevels({ ...levels, [bus]: percent / 100 }));
  };

  if (!open) {
    return (
      <button className="mixer-toggle" onClick={() => setOpen(true)} aria-label={fr.mixer.open}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 3v18M12 3v18M18 3v18" stroke="currentColor" strokeWidth="1.5" />
          <rect x="3.5" y="13" width="5" height="3" rx="1" fill="currentColor" />
          <rect x="9.5" y="6" width="5" height="3" rx="1" fill="currentColor" />
          <rect x="15.5" y="10" width="5" height="3" rx="1" fill="currentColor" />
        </svg>
      </button>
    );
  }

  return (
    <section className="mixer card" aria-label={fr.mixer.title}>
      <h3>{fr.mixer.title}</h3>
      {BUSES.map((bus) => {
        const percent = Math.round(levels[bus] * 100);
        return (
          <label key={bus} className="fader">
            <span className="row">
              <span>{fr.mixer[bus]}</span>
              <span className="muted">{percent} %</span>
            </span>
            <input
              type="range"
              min={0}
              max={MAX_LEVEL * 100}
              step={5}
              value={percent}
              onChange={(e) => change(bus, Number(e.target.value))}
            />
          </label>
        );
      })}
      <div className="row">
        <button onClick={playTableTest}>{fr.mixer.test}</button>
        <button className="quiet" onClick={() => setOpen(false)}>
          {fr.mixer.close}
        </button>
      </div>
    </section>
  );
}
