import { useEffect, useRef, useState } from "react";
import { BUSES, MAX_LEVEL, type MixerLevels } from "./mixer";
import { fr } from "./strings";
import { playTableTest, setTableLevels, tableLevels } from "./tableAudio";

/**
 * The Host's mixer during the Game in a bottom sheet with one fader from 0 to
 * 150% per bus of the Table device's mix and a short test sound
 * Levels apply at once and stay on this phone for the next Games
 */
export function MixerSheet({ onClose }: { onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [levels, setLevels] = useState(tableLevels);

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const change = (bus: keyof MixerLevels, percent: number) => {
    setLevels(setTableLevels({ ...levels, [bus]: percent / 100 }));
  };

  return (
    <dialog
      ref={dialog}
      className="sheet"
      data-ground="parchment"
      aria-labelledby="mixer-title"
      onClose={onClose}
      // A tap on the backdrop lands on the dialog itself outside its content
      onClick={(e) => e.target === dialog.current && dialog.current.close()}
    >
      <div className="sheet-content">
        <h2 id="mixer-title">{fr.mixer.title}</h2>
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
          <button className="quiet" onClick={() => dialog.current?.close()}>
            {fr.mixer.close}
          </button>
        </div>
      </div>
    </dialog>
  );
}
