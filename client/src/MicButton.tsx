import { useEffect, type PointerEvent } from "react";
import { fr } from "./strings";
import type { Call } from "./useCall";

/** Where Space types a character instead of talking. */
function typingIn(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || target.matches("input, textarea, select"));
}

/**
 * How this Player talks: a big button to hold (push-to-talk) or to tap (open
 * mic), and the switch between the two. Shown whenever their mic works, even
 * when nobody can hear them: it decides what happens once someone can.
 */
export function MicButton({ call }: { call: Call }) {
  const { micMode, micLive, wantsToTalk, pressToTalk, releaseToTalk } = call;
  const pushToTalk = micMode === "pushToTalk";

  // Space talks like the button. Anything that takes the page away releases it.
  useEffect(() => {
    if (!pushToTalk) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.code !== "Space") return;
      // Letting go always releases, even once focus moved into a field.
      if (event.type === "keyup") releaseToTalk();
      if (typingIn(event.target)) return;
      // No scrolling, and no pressing a focused button.
      event.preventDefault();
      if (event.type === "keydown" && !event.repeat) pressToTalk();
    };
    const onHidden = () => {
      if (document.visibilityState === "hidden") releaseToTalk();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onKey, true);
    window.addEventListener("blur", releaseToTalk);
    document.addEventListener("visibilitychange", onHidden);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keyup", onKey, true);
      window.removeEventListener("blur", releaseToTalk);
      document.removeEventListener("visibilitychange", onHidden);
      releaseToTalk();
    };
  }, [pushToTalk, pressToTalk, releaseToTalk]);

  const press = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;
    // A finger gets captured on its own: release it so sliding off lets go.
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    pressToTalk();
  };

  return (
    <div className="mic-bar">
      {pushToTalk ? (
        <button
          className="mic-button"
          data-wants={wantsToTalk}
          data-live={micLive}
          onPointerDown={press}
          onPointerUp={releaseToTalk}
          onPointerLeave={releaseToTalk}
          onPointerCancel={releaseToTalk}
          onContextMenu={(event) => event.preventDefault()}
        >
          {fr.call.holdToTalk}
        </button>
      ) : (
        <button className="mic-button" data-wants={wantsToTalk} data-live={micLive} onClick={call.toggleMic}>
          {wantsToTalk ? fr.call.muteMe : fr.call.unmuteMe}
        </button>
      )}
      <label className="toggle mic-mode">
        {fr.call.openMic}
        <input
          type="checkbox"
          role="switch"
          checked={!pushToTalk}
          onChange={(event) => call.chooseMicMode(event.target.checked ? "open" : "pushToTalk")}
        />
      </label>
    </div>
  );
}
