import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import {
  ConnectionState,
  DisconnectReason,
  MediaDeviceFailure,
  Room,
  RoomEvent,
  Track,
  type Participant,
} from "livekit-client";
import type { CallTicket } from "./generated/CallTicket";
import { unlockSpeech } from "./narrator";

/** Why this Player's microphone could not start. */
export type MicProblem = "refused" | "missing" | "unavailable";

/** The microphone is required to play; the camera is optional. */
export type MicStatus = "pending" | "on" | MicProblem;

export type CallStatus = "idle" | "connecting" | "connected" | "failed";

/** Everything a tile needs to show one Player of the call. */
export type CallMember = {
  camera: Track | undefined;
  speaking: boolean;
};

// Any of these may change what a tile shows.
const REDRAW_ON = [
  RoomEvent.ParticipantConnected,
  RoomEvent.ParticipantDisconnected,
  RoomEvent.TrackSubscribed,
  RoomEvent.TrackUnsubscribed,
  RoomEvent.TrackMuted,
  RoomEvent.TrackUnmuted,
  RoomEvent.LocalTrackPublished,
  RoomEvent.LocalTrackUnpublished,
  RoomEvent.ActiveSpeakersChanged,
  RoomEvent.AudioPlaybackStatusChanged,
] as const;

/**
 * The Lobby's LiveKit call. `unlock` must be called inside the "Rejoindre" tap;
 * the call starts once the game server hands over this Player's ticket.
 */
export function useCall(ticket: CallTicket | null) {
  const room = useRef<Room | null>(null);
  const [status, setStatus] = useState<CallStatus>("idle");
  const [mic, setMic] = useState<MicStatus>("pending");
  const [, redraw] = useReducer((n: number) => n + 1, 0);

  const ensureRoom = useCallback(() => {
    if (room.current) return room.current;
    const created = new Room({ adaptiveStream: true, dynacast: true });
    for (const event of REDRAW_ON) created.on(event, redraw);
    // LiveKit already retried and gave up: offer the Player a way back in.
    created.on(RoomEvent.Disconnected, (reason) =>
      setStatus(reason === DisconnectReason.CLIENT_INITIATED ? "idle" : "failed"),
    );
    room.current = created;
    return created;
  }, []);

  /** Runs inside the "Rejoindre" tap: unlocks the Narrator's voice and call audio. */
  const unlock = useCallback(() => {
    unlockSpeech();
    ensureRoom()
      .startAudio()
      .catch(() => {});
  }, [ensureRoom]);

  const enableMedia = useCallback(async () => {
    const me = ensureRoom().localParticipant;
    setMic("pending");
    try {
      await me.enableCameraAndMicrophone();
      setMic("on");
      return;
    } catch {
      // One of the two failed, which fails both: retry each on its own.
    }
    try {
      await me.setMicrophoneEnabled(true);
      setMic("on");
    } catch (error) {
      setMic(micProblem(error));
    }
    // Optional: without it the Player shows as a card back.
    await me.setCameraEnabled(true).catch(() => {});
  }, [ensureRoom]);

  const connect = useCallback(
    async (next: CallTicket) => {
      const current = ensureRoom();
      if (current.state !== ConnectionState.Disconnected) return;
      setStatus("connecting");
      try {
        await current.connect(next.url, next.token);
      } catch {
        setStatus("failed");
        return;
      }
      setStatus("connected");
      await enableMedia();
    },
    [ensureRoom, enableMedia],
  );

  // Each (re)join to the Lobby brings a fresh ticket; only use it when out of the call.
  useEffect(() => {
    if (ticket) void connect(ticket);
  }, [ticket, connect]);

  const leave = useCallback(() => {
    void room.current?.disconnect();
  }, []);

  useEffect(() => leave, [leave]);

  const member = (identity: string, isYou: boolean): CallMember | null => {
    const current = room.current;
    // While LiveKit reconnects, tiles keep showing what they had.
    if (!current || current.state === ConnectionState.Disconnected) return null;
    // LiveKit's view of this Player in the call.
    const peer: Participant | undefined = isYou
      ? current.localParticipant
      : current.remoteParticipants.get(identity);
    if (!peer) return null;
    const camera = peer.getTrackPublication(Track.Source.Camera);
    return {
      camera: camera && !camera.isMuted ? camera.track : undefined,
      speaking: peer.isSpeaking,
    };
  };

  /** Every remote voice this device currently receives. */
  const voices = (): Track[] => {
    const current = room.current;
    if (!current) return [];
    return [...current.remoteParticipants.values()].flatMap((peer) => {
      const track = peer.getTrackPublication(Track.Source.Microphone)?.track;
      return track ? [track] : [];
    });
  };

  return {
    status,
    mic,
    /** Browsers may still block call audio until another tap. */
    audioBlocked: status === "connected" && !(room.current?.canPlaybackAudio ?? true),
    member,
    voices,
    unlock,
    retryMic: enableMedia,
    retryConnect: () => ticket && void connect(ticket),
    startAudio: () => void room.current?.startAudio(),
    leave,
  };
}

export type Call = ReturnType<typeof useCall>;

function micProblem(error: unknown): MicProblem {
  switch (MediaDeviceFailure.getFailure(error)) {
    case MediaDeviceFailure.PermissionDenied:
      return "refused";
    case MediaDeviceFailure.NotFound:
      return "missing";
    default:
      return "unavailable";
  }
}
