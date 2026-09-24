import { useEffect, useRef } from "react";
import type { Track } from "livekit-client";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { fr } from "./strings";
import type { CallMember } from "./useCall";

type Props = {
  view: PlayerView;
  member: (identity: string, isYou: boolean) => CallMember | null;
};

/** One tile per seated Player, in the order they joined. */
export function CallGrid({ view, member }: Props) {
  const count = view.players.length;
  // Fewer, bigger tiles while the Lobby fills up; 12 still fit a portrait phone.
  const size = count <= 4 ? "small" : count <= 9 ? "medium" : "large";
  return (
    <ul className="call-grid" data-size={size}>
      {view.players.map((player) => {
        const isYou = player.id === view.you;
        return (
          <PlayerTile
            key={player.id}
            player={player}
            isYou={isYou}
            isHost={player.id === view.host}
            member={member(String(player.id), isYou)}
          />
        );
      })}
    </ul>
  );
}

function PlayerTile(props: {
  player: PlayerSummary;
  isYou: boolean;
  isHost: boolean;
  member: CallMember | null;
}) {
  const { player, isYou, member } = props;
  const classes = ["tile"];
  if (!player.connected) classes.push("offline");
  if (member?.speaking) classes.push("speaking");
  return (
    <li className={classes.join(" ")}>
      {/* Under the video too: it shows until the first frame arrives. */}
      <CardBack />
      {member?.camera && <Video track={member.camera} mirrored={isYou} />}
      <div className="tile-label">
        <span className="name">
          {player.name}
          {isYou && <span className="muted"> ({fr.lobby.you})</span>}
        </span>
        {props.isHost && <span className="badge">{fr.lobby.host}</span>}
        {!player.connected && <span className="muted">{fr.lobby.offline}</span>}
      </div>
    </li>
  );
}

/** Plays `track` in the returned element for as long as it is mounted. */
function useAttached<E extends HTMLMediaElement>(track: Track) {
  const ref = useRef<E>(null);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    track.attach(element);
    return () => {
      track.detach(element);
    };
  }, [track]);
  return ref;
}

function Video({ track, mirrored }: { track: Track; mirrored: boolean }) {
  const ref = useAttached<HTMLVideoElement>(track);
  // Muted: the Player's voice plays through a separate <Voice>.
  return <video ref={ref} className={mirrored ? "mirrored" : ""} autoPlay playsInline muted />;
}

/** Plays one remote Player's voice. */
export function Voice({ track }: { track: Track }) {
  const ref = useAttached<HTMLAudioElement>(track);
  return <audio ref={ref} autoPlay />;
}

/** A Player without a camera shows as the back of a Role card. */
function CardBack() {
  return (
    <div className="card-back" aria-hidden="true">
      <svg viewBox="0 0 40 40" className="emblem">
        <circle cx="20" cy="20" r="17" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M24 9a11 11 0 1 0 7 17A13 13 0 0 1 24 9z" fill="currentColor" />
      </svg>
    </div>
  );
}
