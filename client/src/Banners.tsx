import type { ReactNode } from "react";
import { fr } from "./strings";

/** This phone has lost the server and is trying again. */
export function ConnectionLost() {
  return (
    <p className="banner alert" role="status">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
        <path d="M2 8.5a15 15 0 0 1 20 0M5.5 12a10 10 0 0 1 13 0M9 15.5a5 5 0 0 1 6 0M3 3l18 18" />
        <circle cx="12" cy="19.5" r="1" fill="currentColor" />
      </svg>
      <span>{fr.join.connectionLost}</span>
    </p>
  );
}

/** The Game is Paused until the Host, named `hostName`, takes their seat back. */
export function Paused({ hostName }: { hostName: string }) {
  return (
    <p className="banner" role="status">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="6" y="5" width="4" height="14" rx="1" fill="currentColor" />
        <rect x="14" y="5" width="4" height="14" rx="1" fill="currentColor" />
      </svg>
      <span>{fr.game.paused(hostName)}</span>
    </p>
  );
}

/** A failure the Player can act on, read out as soon as it shows */
export function Alert({ children }: { children: ReactNode }) {
  return (
    <p className="banner alert" role="alert">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9" />
        <path d="M12 7.5v5.5" />
        <circle cx="12" cy="16.5" r="1" fill="currentColor" />
      </svg>
      <span>{children}</span>
    </p>
  );
}
