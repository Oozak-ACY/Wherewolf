import { useEffect, useRef } from "react";
import type { Track } from "livekit-client";
import type { PlayerView } from "./generated/PlayerView";
import { CardBack, gridSize, TileLabel, tileClasses } from "./PlayerGrid";
import type { CallMember } from "./useCall";

type Props = {
  view: PlayerView;
  member: (identity: string, isYou: boolean) => CallMember | null;
};

/** One tile per seated Player, in the order they joined, with their camera. */
export function CallGrid({ view, member }: Props) {
  return (
    <ul className="call-grid" data-size={gridSize(view.players.length)}>
      {view.players.map((player) => {
        const isYou = player.id === view.you;
        const media = member(String(player.id), isYou);
        const classes = tileClasses(player);
        if (media?.speaking) classes.push("speaking");
        return (
          <li key={player.id} className={classes.join(" ")}>
            {/* Under the video too: it shows until the first frame arrives. */}
            <CardBack />
            {media?.camera && <Video track={media.camera} mirrored={isYou} />}
            <TileLabel view={view} player={player} />
          </li>
        );
      })}
    </ul>
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
