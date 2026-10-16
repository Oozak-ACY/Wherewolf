// The seat token lives in this browser only: reopening a Lobby link here
// reclaims the same seat. No account needed. The display name, the mic mode
// and the Host's mixer levels are remembered here too.

import type { MicMode } from "./micControl";
import { clampLevels, DEFAULT_LEVELS, type MixerLevels } from "./mixer";

const TOKEN_KEY = "wherewolf.seatToken";
const NAME_KEY = "wherewolf.name";
const LOBBY_KEY = "wherewolf.seatedIn";
const MIC_MODE_KEY = "wherewolf.micMode";
const LEVELS_KEY = "wherewolf.mixerLevels";

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

/** How this Player talks on this browser: push-to-talk until they choose. */
export function rememberedMicMode(): MicMode {
  return read(MIC_MODE_KEY) === "open" ? "open" : "pushToTalk";
}

export function rememberMicMode(mode: MicMode): void {
  write(MIC_MODE_KEY, mode);
}

/** The levels the Host left the Table device's faders at, on this phone. */
export function rememberedLevels(): MixerLevels {
  const stored = read(LEVELS_KEY);
  if (!stored) return DEFAULT_LEVELS;
  try {
    const levels: unknown = JSON.parse(stored);
    return typeof levels === "object" && levels !== null ? clampLevels(levels) : DEFAULT_LEVELS;
  } catch {
    return DEFAULT_LEVELS;
  }
}

export function rememberLevels(levels: MixerLevels): void {
  write(LEVELS_KEY, JSON.stringify(levels));
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
