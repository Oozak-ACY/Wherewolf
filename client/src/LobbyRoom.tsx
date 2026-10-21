import { useState, type ReactNode } from "react";
import type { PlayerView } from "./generated/PlayerView";
import type { Settings } from "./generated/Settings";
import { joinLink } from "./joinLink";
import { initial, playerTag } from "./lobbyRoom";
import { QrCodeView } from "./QrCodeView";
import { MoreSettings, RoleCards, StartFooter } from "./SettingsPanel";
import { fr } from "./strings";

/** The Lobby before a Game: the code to invite the table, the Players, the Settings and the start. */
export function LobbyRoom(props: {
  view: PlayerView;
  banners: ReactNode;
  onSettingsChange: (settings: Settings) => void;
  onStart: () => void;
  onLeave: () => void;
}) {
  const { view } = props;
  const url = joinLink(window.location.origin, view.code);
  const [copied, setCopied] = useState(false);
  const [qrOpen, setQrOpen] = useState(false);
  const isHost = view.you === view.host;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
    } catch {
      // Clipboard refused (insecure context): the link stays visible to copy by hand.
    }
  };

  const share = () => navigator.share({ title: fr.lobby.shareTitle, url }).catch(() => {});

  return (
    <div className="room">
      <header className="room-header">
        <div className="room-code">
          <span>{fr.lobby.code}</span>
          <strong>{view.code}</strong>
        </div>
        <div className="room-actions">
          {isHost && (
            <button className="icon" aria-label={fr.lobby.showQrCode} onClick={() => setQrOpen(true)}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                <rect x="3" y="3" width="7" height="7" />
                <rect x="14" y="3" width="7" height="7" />
                <rect x="3" y="14" width="7" height="7" />
                <path d="M14 14h3v3h-3zM20 14v7M14 20h3" />
              </svg>
            </button>
          )}
          {"share" in navigator && (
            <button className="icon" aria-label={fr.lobby.share} onClick={share}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                <circle cx="18" cy="5" r="3" />
                <circle cx="6" cy="12" r="3" />
                <circle cx="18" cy="19" r="3" />
                <path d="M8.6 13.5l6.8 4M15.4 6.5l-6.8 4" />
              </svg>
            </button>
          )}
        </div>
      </header>

      <div className="room-body">
        {props.banners}

        <section className="stack">
          <div className="section-head">
            <h2>{fr.lobby.players(view.players.length)}</h2>
            <span className="muted">{fr.lobby.minimum}</span>
          </div>
          <ul className="player-tiles">
            {view.players.map((player) => (
              <li key={player.id} className="player-tile" data-offline={!player.connected}>
                <span className="avatar" aria-hidden="true">
                  {initial(player.name)}
                </span>
                <span className="player-text">
                  <span className="player-name">{player.name}</span>
                  <span className="player-tag">{playerTag(view, player)}</span>
                </span>
              </li>
            ))}
          </ul>
        </section>

        <p className="star-divider" aria-hidden="true">
          <svg viewBox="0 0 16 16">
            <path d="M8 1l2 5 5 2-5 2-2 5-2-5-5-2 5-2z" />
          </svg>
        </p>

        <RoleCards view={view} onChange={props.onSettingsChange} />
        <MoreSettings view={view} onChange={props.onSettingsChange} />

        <section className="invite">
          <p className="link">{url}</p>
          <button className="quiet" onClick={copy}>
            {copied ? fr.lobby.copied : fr.lobby.copy}
          </button>
          <button className="quiet" onClick={props.onLeave}>
            {fr.lobby.leave}
          </button>
        </section>
      </div>

      <StartFooter view={view} onStart={props.onStart} />
      {isHost && qrOpen && <QrCodeView link={url} code={view.code} onClose={() => setQrOpen(false)} />}
    </div>
  );
}
