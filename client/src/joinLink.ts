// The join link of a Lobby: shared by hand, copied, or scanned from the Host's
// QR code. Opening it lands on that Lobby's join screen.

const LOBBY_PATH = /^\/l\/([A-Za-z0-9]+)\/?$/;

export function joinLink(origin: string, code: string): string {
  return `${origin}/l/${code}`;
}

/** The Lobby code a path opens, if it opens one. */
export function lobbyCodeFromPath(path: string): string | undefined {
  return LOBBY_PATH.exec(path)?.[1]?.toUpperCase();
}
