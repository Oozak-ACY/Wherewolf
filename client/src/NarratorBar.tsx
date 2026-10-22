import { useEffect, useState } from "react";
import { MixerSheet } from "./MixerSheet";
import type { NarratorBar as Bar } from "./narratorBar";
import { fr } from "./strings";

/**
 * Atop every in-Game screen the Phase and the Narrator's line and the time
 * left with the Player's card and the mixer on the Table device
 */
export function NarratorBar(props: { bar: Bar; endsAt: number | null; paused: boolean; onOpenCard: () => void }) {
  const { bar } = props;
  const [mixerOpen, setMixerOpen] = useState(false);
  return (
    <header className="narrator-bar" data-phase={bar.phase}>
      {bar.phase === "night" ? <MoonIcon /> : <SunIcon />}
      <div className="narrator-text">
        <span className="narrator-label">{bar.label}</span>
        <span className="narrator-line" aria-live="polite">{bar.line}</span>
      </div>
      {props.endsAt !== null && <Countdown endsAt={props.endsAt} paused={props.paused} />}
      {bar.card && (
        <button className="icon" onClick={props.onOpenCard} aria-label={fr.game.myCard}>
          <CardBackIcon />
        </button>
      )}
      {bar.mixer && (
        <button className="icon" onClick={() => setMixerOpen(true)} aria-label={fr.mixer.open}>
          <VolumeIcon />
        </button>
      )}
      {mixerOpen && <MixerSheet onClose={() => setMixerOpen(false)} />}
    </header>
  );
}

/** Time left until `endsAt` as m:ss and still while the Game is Paused */
function Countdown({ endsAt, paused }: { endsAt: number; paused: boolean }) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    if (paused) return;
    const tick = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(tick);
  }, [endsAt, paused]);
  const seconds = Math.max(0, Math.ceil((endsAt - now) / 1000));
  const text = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  return (
    <span className="countdown" role="timer">
      {text}
    </span>
  );
}

function SunIcon() {
  return (
    <svg className="phase-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
      <circle cx="12" cy="12" r="4.5" fill="currentColor" />
      <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg className="phase-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z" fill="currentColor" />
    </svg>
  );
}

/** A generic card back and never the Player's own art */
function CardBackIcon() {
  return (
    <svg viewBox="0 0 18 32" width="16" height="28" aria-hidden="true">
      <rect x="1" y="1" width="16" height="30" rx="2" fill="var(--ww-night)" stroke="currentColor" strokeWidth="1.5" />
      <rect x="3.5" y="3.5" width="11" height="25" rx="1" fill="none" stroke="currentColor" strokeWidth="0.75" />
      <path d="M10.5 11.5a4.5 4.5 0 1 0 2.9 7 5.3 5.3 0 0 1-2.9-7z" fill="currentColor" />
    </svg>
  );
}

function VolumeIcon() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
      <path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" strokeLinejoin="round" />
      <path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12" strokeLinecap="round" />
    </svg>
  );
}
