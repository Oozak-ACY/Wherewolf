import { useCallback, useEffect, useRef, useState } from "react";
import type { PlayerView } from "./generated/PlayerView";
import type { Rejection } from "./generated/Rejection";
import { openLobbySocket, parse, send } from "./server";
import { rememberName, seatToken } from "./seat";

export type LobbyStatus = "idle" | "connecting" | "joined" | "reconnecting" | "notFound";

const RETRY_DELAYS_MS = [500, 1000, 2000, 5000];

/** The connection to one Lobby. Holds no game rule: it only shows the server's view. */
export function useLobby(code: string) {
  const [status, setStatus] = useState<LobbyStatus>("idle");
  const [view, setView] = useState<PlayerView | null>(null);
  const [rejection, setRejection] = useState<Rejection | null>(null);

  const socket = useRef<WebSocket | null>(null);
  const name = useRef("");
  // True from a successful join until the Player leaves: a dropped socket is
  // then reopened, and joining again with the same token reclaims the seat.
  const seated = useRef(false);
  const attempt = useRef(0);
  const retry = useRef<number | undefined>(undefined);

  const connect = useCallback(() => {
    const ws = openLobbySocket(code);
    socket.current = ws;
    ws.onopen = () => send(ws, { type: "join", name: name.current, seatToken: seatToken() });
    ws.onmessage = (event) => {
      const message = parse(event.data);
      switch (message.type) {
        case "view":
          seated.current = true;
          attempt.current = 0;
          setView(message.view);
          setRejection(null);
          setStatus("joined");
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
  }, [code]);

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
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) send(ws, { type: "leave" });
    ws?.close();
    socket.current = null;
  }, []);

  useEffect(
    () => () => {
      seated.current = false;
      window.clearTimeout(retry.current);
      socket.current?.close();
    },
    [],
  );

  return { status, view, rejection, join, leave };
}
