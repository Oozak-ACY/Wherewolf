// The seat token lives in this browser only: reopening a Lobby link here
// reclaims the same seat. No account needed.

const TOKEN_KEY = "wherewolf.seatToken";
const NAME_KEY = "wherewolf.name";
const LOBBY_KEY = "wherewolf.seatedIn";

export function seatToken(): string {
  const stored = read(TOKEN_KEY);
  if (stored) return stored;
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  const token = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  write(TOKEN_KEY, token);
  return token;
}

export function rememberedName(): string {
  return read(NAME_KEY) ?? "";
}

export function rememberName(name: string): void {
  write(NAME_KEY, name);
}

/** The Lobby this browser holds a seat in, so reopening its link takes it back. */
export function seatedLobby(): string | null {
  return read(LOBBY_KEY);
}

export function rememberSeatedLobby(code: string): void {
  write(LOBBY_KEY, code);
}

export function forgetSeatedLobby(code: string): void {
  if (read(LOBBY_KEY) === code) remove(LOBBY_KEY);
}

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Private browsing: the seat simply won't survive a reload.
  }
}

function remove(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // Nothing could have been stored.
  }
}
