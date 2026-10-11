import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import {
  ConnectionState,
  DisconnectReason,
  MediaDeviceFailure,
  Room,
  RoomEvent,
  Track,
  type LocalTrack,
  type Participant,
} from "livekit-client";
import type { CallTicket } from "./generated/CallTicket";
import type { PlayerId } from "./generated/PlayerId";
import { createMicControl, type MicMode, type NarratorSpeaking } from "./micControl";
import { unlockSpeech } from "./narrator";
import { rememberedMicMode, rememberMicMode } from "./seat";

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
 *
 * `audience` is who may see and hear this Player right now, as the Narrator
 * decides: only they may subscribe to this device's camera and microphone
 * (ADR 0001, the publisher-side lever). `null` until the first view arrives.
 *
 * `refreshTicket` asks the game server for a new ticket, whose permissions
 * match the current Moment, to get back into the call after losing it.
 */
export function useCall(
  ticket: CallTicket | null,
  audience: PlayerId[] | null,
  refreshTicket: () => void,
) {
  const room = useRef<Room | null>(null);
  const allowed = useRef<PlayerId[] | null>(audience);
  const [status, setStatus] = useState<CallStatus>("idle");
  const [mic, setMic] = useState<MicStatus>("pending");
  // This Player's microphone track, published or about to be.
  const micTrack = useRef<LocalTrack | null>(null);
  // Muting is a track mute: never revoke `canPublish` nor republish (ADR 0001).
  const [control] = useState(() =>
    createMicControl(rememberedMicMode(), (live) => {
      setMicLive(live);
      void (live ? micTrack.current?.unmute() : micTrack.current?.mute())?.catch(() => {});
    }),
  );
  const [micMode, setMicMode] = useState<MicMode>(() => control.mode());
  const [micLive, setMicLive] = useState(() => control.live());
  const [wantsToTalk, setWantsToTalk] = useState(() => control.wantsToTalk());
  // Browsers only let sound play after a tap. False after a reload that took
  // the seat back on its own, until the Player taps to enable sound.
  const [unlocked, setUnlocked] = useState(false);
  const [, redraw] = useReducer((n: number) => n + 1, 0);

  /** Lets only the current audience subscribe to this device's tracks. */
  const restrictAudience = useCallback(() => {
    const current = room.current;
    if (!allowed.current || current?.state !== ConnectionState.Connected) return;
    current.localParticipant.setTrackSubscriptionPermissions(
      false,
      allowed.current.map((id) => ({ participantIdentity: String(id), allowAll: true })),
    );
  }, []);

  const ensureRoom = useCallback(() => {
    if (room.current) return room.current;
    const created = new Room({ adaptiveStream: true, dynacast: true });
    for (const event of REDRAW_ON) created.on(event, redraw);
    // LiveKit already retried and gave up: offer the Player a way back in.
    created.on(RoomEvent.Disconnected, (reason) =>
      setStatus(reason === DisconnectReason.CLIENT_INITIATED ? "idle" : "failed"),
    );
    // A reconnect may start over: restrict the audience again.
    created.on(RoomEvent.Reconnected, restrictAudience);
    room.current = created;
    return created;
  }, [restrictAudience]);

  /** Runs inside the "Rejoindre" tap: unlocks the Narrator's voice and call audio. */
  const unlock = useCallback(() => {
    setUnlocked(true);
    unlockSpeech();
    ensureRoom()
      .startAudio()
      .catch(() => {});
  }, [ensureRoom]);

  const enableMedia = useCallback(async () => {
    const me = ensureRoom().localParticipant;
    setMic("pending");
    let tracks: LocalTrack[] = [];
    try {
      // Both at once, so a browser asks only once. A retry may already have the camera.
      tracks = await me.createTracks({ audio: true, video: !me.getTrackPublication(Track.Source.Camera) });
    } catch {
      // One of the two failed, which fails both: retry each on its own.
    }
    let audio = tracks.find((track) => track.kind === Track.Kind.Audio);
    try {
      audio ??= (await me.createTracks({ audio: true }))[0];
      micTrack.current = audio;
      // Published muted unless the Player already wants to talk: no live audio escapes.
      if (!control.live()) await audio.mute();
      await me.publishTrack(audio);
      setMic("on");
    } catch (error) {
      audio?.stop();
      setMic(micProblem(error));
    }
    // Optional: without it the Player shows as a card back.
    const camera = tracks.find((track) => track.kind === Track.Kind.Video);
    await (camera ? me.publishTrack(camera) : me.setCameraEnabled(true)).catch(() => {});
  }, [ensureRoom, control]);

  // The key changes only when the audience does, not on every view.
  const audienceKey = audience?.join(",") ?? null;
  useEffect(() => {
    allowed.current = audience;
    restrictAudience();
  }, [audienceKey, restrictAudience]);

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
      restrictAudience();
      await enableMedia();
    },
    [ensureRoom, enableMedia, restrictAudience],
  );

  // Each (re)join to the Lobby brings a fresh ticket; only use it when out of the call.
  useEffect(() => {
    if (ticket) void connect(ticket);
  }, [ticket, connect]);

  // LiveKit gave up (a locked screen, another app, a lost network): ask for a
  // fresh ticket now, and again whenever the page comes back.
  useEffect(() => {
    if (status !== "failed") return;
    refreshTicket();
    const resume = () => {
      if (document.visibilityState === "visible") refreshTicket();
    };
    document.addEventListener("visibilitychange", resume);
    window.addEventListener("online", resume);
    return () => {
      document.removeEventListener("visibilitychange", resume);
      window.removeEventListener("online", resume);
    };
  }, [status, refreshTicket]);

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

  /** Mutes this Player's microphone while the Narrator speaks; only the Player brings it back. */
  const narratorSpeaking: NarratorSpeaking = useMemo(
    () => ({ started: control.narratorStarted, ended: control.narratorEnded }),
    [control],
  );

  /** What the Player does to talk, shown on the button even while the Narrator mutes them. */
  const talk = useMemo(() => {
    const andShow = (act: () => void) => () => {
      act();
      setWantsToTalk(control.wantsToTalk());
    };
    return {
      press: andShow(control.press),
      release: andShow(control.release),
      toggle: andShow(control.toggle),
      chooseMode: (mode: MicMode) => {
        andShow(() => control.setMode(mode))();
        setMicMode(mode);
        rememberMicMode(mode);
      },
    };
  }, [control]);

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
    /** Sound needs another tap: the seat came back without one, or the browser blocks call audio. */
    audioBlocked:
      !unlocked ||
      (status === "connected" && !(room.current?.canPlaybackAudio ?? true)),
    member,
    voices,
    narratorSpeaking,
    /** How this Player talks, remembered on this browser. */
    micMode,
    chooseMicMode: talk.chooseMode,
    /** Whether this Player's published mic is live right now. */
    micLive,
    /** Whether the Player wants to talk; the Narrator may still keep the mic muted. */
    wantsToTalk,
    /** Push-to-talk: the button or Space went down, or up. */
    pressToTalk: talk.press,
    releaseToTalk: talk.release,
    /** Open mic: the Player mutes or unmutes themselves. */
    toggleMic: talk.toggle,
    unlock,
    retryMic: enableMedia,
    retryConnect: refreshTicket,
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
