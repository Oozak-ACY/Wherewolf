import { useEffect, useRef, useState } from "react";
import { BUSES, MAX_LEVEL, type MixerLevels } from "./mixer";
import { fr } from "./strings";
import { playTableTest, setTableLevels, tableLevels } from "./tableAudio";

/**
 * The options menu during the Game in a bottom sheet with leaving the salon
 * for every Player after a confirmation and the mixer on the Table device
 */
export function OptionsSheet(props: { mixer: boolean; onLeave: () => void; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const close = () => dialog.current?.close();

  return (
    <dialog
      ref={dialog}
      className="sheet"
      data-ground="parchment"
      aria-labelledby="options-title"
      onClose={props.onClose}
      // A tap on the backdrop lands on the dialog itself outside its content
      onClick={(e) => e.target === dialog.current && close()}
    >
      {leaving ? (
        <div className="sheet-content">
          <h2 id="options-title">{fr.options.leaveTitle}</h2>
          <p className="muted">{fr.options.leaveHint}</p>
          <div className="row">
            <button className="primary" onClick={props.onLeave}>
              {fr.options.leaveConfirm}
            </button>
            <button className="quiet" onClick={() => setLeaving(false)}>
              {fr.options.leaveCancel}
            </button>
          </div>
        </div>
      ) : (
        <div className="sheet-content">
          <h2 id="options-title">{fr.options.title}</h2>
          {props.mixer && <Mixer />}
          <button className="quiet" onClick={() => setLeaving(true)}>
            {fr.lobby.leave}
          </button>
          <button className="quiet" onClick={close}>
            {fr.options.close}
          </button>
        </div>
      )}
    </dialog>
  );
}

/**
 * The Host's mixer with one fader from 0 to 150% per bus of the Table
 * device's mix and a short test sound
 * Levels apply at once and stay on this phone for the next Games
 */
function Mixer() {
  const [levels, setLevels] = useState(tableLevels);

  const change = (bus: keyof MixerLevels, percent: number) => {
    setLevels(setTableLevels({ ...levels, [bus]: percent / 100 }));
  };

  return (
    <section className="sheet-section" aria-labelledby="mixer-title">
      <h3 id="mixer-title">{fr.mixer.title}</h3>
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
      <button onClick={playTableTest}>{fr.mixer.test}</button>
    </section>
  );
}
