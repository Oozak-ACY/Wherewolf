import { useCallback, useEffect, useRef, useState } from "react";
import type { CallTicket } from "./generated/CallTicket";
import type { ClientMessage } from "./generated/ClientMessage";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerView } from "./generated/PlayerView";
import type { Rejection } from "./generated/Rejection";
import type { Settings } from "./generated/Settings";
import { openLobbySocket, parse, send } from "./server";
import {
  forgetSeatedLobby,
  rememberedName,
  rememberName,
  rememberSeatedLobby,
  seatedLobby,
  seatToken,
} from "./seat";

export type LobbyStatus = "idle" | "connecting" | "joined" | "reconnecting" | "notFound";

const RETRY_DELAYS_MS = [500, 1000, 2000, 5000];

/** The connection to one Lobby. Holds no game rule: it only shows the server's view. */
export function useLobby(code: string) {
  const [status, setStatus] = useState<LobbyStatus>("idle");
  const [view, setView] = useState<PlayerView | null>(null);
  // When the Game's current moment ends, in this device's clock (`Date.now()`).
  const [endsAt, setEndsAt] = useState<number | null>(null);
  const [rejection, setRejection] = useState<Rejection | null>(null);
  const [ticket, setTicket] = useState<CallTicket | null>(null);

  const socket = useRef<WebSocket | null>(null);
  const name = useRef("");
  // True from a successful join until the Player leaves: a dropped socket is
  // then reopened, and joining again with the same token reclaims the seat.
  const seated = useRef(false);
  const attempt = useRef(0);
  const retry = useRef<number | undefined>(undefined);

  const sendJoin = useCallback(
    (ws: WebSocket) => send(ws, { type: "join", name: name.current, seatToken: seatToken() }),
    [],
  );

  const connect = useCallback(() => {
    const ws = openLobbySocket(code);
    socket.current = ws;
    ws.onopen = () => sendJoin(ws);
    ws.onmessage = (event) => {
      const message = parse(event.data);
      switch (message.type) {
        case "view":
          seated.current = true;
          rememberSeatedLobby(code);
          attempt.current = 0;
          setView(message.view);
          setEndsAt(message.endsInMs === null ? null : Date.now() + message.endsInMs);
          setRejection(null);
          setStatus("joined");
          break;
        case "call":
          setTicket(message.ticket);
          break;
        case "rejected":
          setRejection(message.reason);
          if (!seated.current) {
            ws.close();
            setStatus("idle");
          }
          break;
        case "lobbyNotFound":
          seated.current = false;
          forgetSeatedLobby(code);
          ws.close();
          setStatus("notFound");
          break;
      }
    };
    ws.onclose = () => {
      if (socket.current !== ws || !seated.current) return;
      setStatus("reconnecting");
      const delay = RETRY_DELAYS_MS[Math.min(attempt.current, RETRY_DELAYS_MS.length - 1)];
      attempt.current += 1;
      retry.current = window.setTimeout(connect, delay);
    };
  }, [code, sendJoin]);

  const join = useCallback(
    (displayName: string) => {
      name.current = displayName.trim();
      rememberName(name.current);
      setRejection(null);
      setStatus("connecting");
      connect();
    },
    [connect],
  );

  const leave = useCallback(() => {
    seated.current = false;
    forgetSeatedLobby(code);
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) send(ws, { type: "leave" });
    ws?.close();
    socket.current = null;
  }, [code]);

  // Back from a locked screen, another app or a lost network: reconnect now
  // rather than at the end of the retry backoff.
  useEffect(() => {
    const resume = () => {
      if (!seated.current) return;
      const state = socket.current?.readyState;
      if (state === WebSocket.OPEN || state === WebSocket.CONNECTING) return;
      window.clearTimeout(retry.current);
      attempt.current = 0;
      connect();
    };
    document.addEventListener("visibilitychange", resume);
    window.addEventListener("online", resume);
    window.addEventListener("pageshow", resume);
    return () => {
      document.removeEventListener("visibilitychange", resume);
      window.removeEventListener("online", resume);
      window.removeEventListener("pageshow", resume);
    };
  }, [connect]);

  // Reopening the link of a Lobby this browser is seated in takes the seat
  // back at once, mid-Game included: no form to fill while the Game goes on.
  useEffect(() => {
    const remembered = rememberedName();
    if (seatedLobby() === code && remembered.trim()) join(remembered);
  }, [code, join]);

  /** Joins again over the open socket, for a fresh call ticket matching the current Moment. */
  const refreshTicket = useCallback(() => {
    const ws = socket.current;
    if (seated.current && ws?.readyState === WebSocket.OPEN) sendJoin(ws);
  }, [sendJoin]);

  const command = useCallback((message: ClientMessage) => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) send(ws, message);
  }, []);

  const updateSettings = useCallback(
    (settings: Settings) => command({ type: "updateSettings", settings }),
    [command],
  );
  const start = useCallback(() => command({ type: "start" }), [command]);
  const inspect = useCallback(
    (player: PlayerId) => command({ type: "inspect", player }),
    [command],
  );
  const pickVictim = useCallback(
    (victim: PlayerId) => command({ type: "pickVictim", victim }),
    [command],
  );
  const heal = useCallback(() => command({ type: "heal" }), [command]);
  const pass = useCallback(() => command({ type: "pass" }), [command]);
  const poison = useCallback(
    (player: PlayerId) => command({ type: "poison", player }),
    [command],
  );
  const elect = useCallback(
    (candidate: PlayerId) => command({ type: "elect", candidate }),
    [command],
  );
  const breakTie = useCallback(
    (player: PlayerId) => command({ type: "breakTie", player }),
    [command],
  );
  const nameSuccessor = useCallback(
    (player: PlayerId) => command({ type: "nameSuccessor", player }),
    [command],
  );
  const shoot = useCallback(
    (player: PlayerId) => command({ type: "shoot", player }),
    [command],
  );
  const vote = useCallback(
    (designated: PlayerId | null) => command({ type: "vote", designated }),
    [command],
  );
  const playAgain = useCallback(() => command({ type: "playAgain" }), [command]);

  useEffect(
    () => () => {
      seated.current = false;
      window.clearTimeout(retry.current);
      socket.current?.close();
    },
    [],
  );

  return {
    status,
    view,
    endsAt,
    rejection,
    ticket,
    refreshTicket,
    join,
    leave,
    updateSettings,
    start,
    inspect,
    pickVictim,
    heal,
    pass,
    poison,
    vote,
    shoot,
    elect,
    breakTie,
    nameSuccessor,
    playAgain,
  };
}
