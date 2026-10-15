import type { ClientMessage } from "./generated/ClientMessage";
import type { CreatedLobby } from "./generated/CreatedLobby";
import type { LiveLineRequest } from "./generated/LiveLineRequest";
import type { ServerMessage } from "./generated/ServerMessage";
import { seatToken } from "./seat";

// Empty in development (the Vite proxy forwards /api to the local server).
const SERVER_URL: string = import.meta.env.VITE_SERVER_URL ?? "";

export async function createLobby(): Promise<string> {
  const response = await fetch(`${SERVER_URL}/api/lobbies`, { method: "POST" });
  if (!response.ok) throw new Error(`create lobby: ${response.status}`);
  const created: CreatedLobby = await response.json();
  return created.code;
}

/**
 * The MP3 of a live Narrator line, voiced by the server for the Table device.
 * Rejects when the server can't voice it (no ElevenLabs key, or a failure).
 */
export async function fetchLiveLine(code: string, text: string): Promise<ArrayBuffer> {
  const request: LiveLineRequest = { seatToken: seatToken(), text };
  const response = await fetch(`${SERVER_URL}/api/lobbies/${encodeURIComponent(code)}/live-line`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(request),
  });
  if (!response.ok) throw new Error(`live line: ${response.status}`);
  return response.arrayBuffer();
}

export function openLobbySocket(code: string): WebSocket {
  const base = new URL(SERVER_URL || window.location.origin);
  base.protocol = base.protocol === "https:" ? "wss:" : "ws:";
  base.pathname = `/api/lobbies/${encodeURIComponent(code)}/ws`;
  return new WebSocket(base);
}

export function send(socket: WebSocket, message: ClientMessage): void {
  socket.send(JSON.stringify(message));
}

export function parse(data: unknown): ServerMessage {
  return JSON.parse(String(data)) as ServerMessage;
}
