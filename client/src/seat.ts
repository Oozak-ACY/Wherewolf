// The seat token lives in this browser only: reopening a Lobby link here
// reclaims the same seat. No account needed.

const TOKEN_KEY = "wherewolf.seatToken";
const NAME_KEY = "wherewolf.name";

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
