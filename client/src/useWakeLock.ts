import { useEffect } from "react";

/**
 * Keeps the screen on while `active`. A phone that locks mid-Game would drop
 * out of the call and miss the Narrator. The browser releases the lock when
 * the page is hidden, so it is taken again each time the page comes back.
 */
export function useWakeLock(active: boolean) {
  useEffect(() => {
    if (!active || !("wakeLock" in navigator)) return;
    let lock: WakeLockSentinel | null = null;
    let released = false;
    const acquire = async () => {
      if (document.visibilityState !== "visible" || (lock && !lock.released)) return;
      try {
        const next = await navigator.wakeLock.request("screen");
        if (released) void next.release();
        else lock = next;
      } catch {
        // Refused (low battery, unsupported context): the screen may dim.
      }
    };
    void acquire();
    document.addEventListener("visibilitychange", acquire);
    return () => {
      released = true;
      document.removeEventListener("visibilitychange", acquire);
      void lock?.release();
    };
  }, [active]);
}
