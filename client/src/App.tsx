import { useEffect, useState, type FormEvent } from "react";
import { CallGrid, Voice } from "./CallGrid";
import type { PlayerView } from "./generated/PlayerView";
import { rememberedName } from "./seat";
import { createLobby } from "./server";
import { fr } from "./strings";
import { useCall, type Call } from "./useCall";
import { useLobby } from "./useLobby";

const LOBBY_PATH = /^\/l\/([A-Za-z0-9]+)\/?$/;

function lobbyUrl(code: string): string {
  return `${window.location.origin}/l/${code}`;
}

function usePath(): [string, (path: string) => void] {
  const [path, setPath] = useState(window.location.pathname);
  useEffect(() => {
    const onPop = () => setPath(window.location.pathname);
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);
  const navigate = (to: string) => {
    window.history.pushState(null, "", to);
    setPath(to);
  };
  return [path, navigate];
}

export function App() {
  const [path, navigate] = usePath();
  const code = LOBBY_PATH.exec(path)?.[1]?.toUpperCase();
  return (
    <main className="screen">
      {code ? (
        <LobbyScreen key={code} code={code} onExit={() => navigate("/")} />
      ) : (
        <Home onOpenLobby={(c) => navigate(`/l/${c}`)} />
      )}
    </main>
  );
}

function Home({ onOpenLobby }: { onOpenLobby: (code: string) => void }) {
  const [creating, setCreating] = useState(false);
  const [failed, setFailed] = useState(false);
  const [code, setCode] = useState("");

  const create = async () => {
    setCreating(true);
    setFailed(false);
    try {
      onOpenLobby(await createLobby());
    } catch {
      setFailed(true);
      setCreating(false);
    }
  };

  const goTo = (event: FormEvent) => {
    event.preventDefault();
    const trimmed = code.trim().toUpperCase();
    if (trimmed) onOpenLobby(trimmed);
  };

  return (
    <>
      <header className="hero">
        <h1>{fr.appName}</h1>
        <p>{fr.tagline}</p>
      </header>
      <button className="primary" onClick={create} disabled={creating}>
        {creating ? fr.home.creating : fr.home.create}
      </button>
      {failed && <p className="error">{fr.home.createFailed}</p>}
      <form className="stack" onSubmit={goTo}>
        <label htmlFor="code">{fr.home.joinWithCode}</label>
        <div className="row">
          <input
            id="code"
            value={code}
            onChange={(e) => setCode(e.target.value)}
            placeholder={fr.home.codePlaceholder}
            autoCapitalize="characters"
            autoComplete="off"
            maxLength={8}
          />
          <button type="submit" disabled={!code.trim()}>
            {fr.home.go}
          </button>
        </div>
      </form>
    </>
  );
}

function LobbyScreen({ code, onExit }: { code: string; onExit: () => void }) {
  const lobby = useLobby(code);
  const call = useCall(lobby.ticket);

  if (lobby.status === "notFound") {
    return (
      <>
        <p className="error">{fr.join.lobbyNotFound}</p>
        <button onClick={onExit}>{fr.join.backHome}</button>
      </>
    );
  }

  if (lobby.view) {
    return (
      <Lobby
        view={lobby.view}
        call={call}
        reconnecting={lobby.status === "reconnecting"}
        onLeave={() => {
          call.leave();
          lobby.leave();
          onExit();
        }}
      />
    );
  }

  return (
    <JoinForm
      code={code}
      connecting={lobby.status === "connecting"}
      error={lobby.rejection && fr.rejection[lobby.rejection]}
      onJoin={(name) => {
        // Still inside the "Rejoindre" tap: the only moment iOS lets us unlock audio.
        call.unlock();
        lobby.join(name);
      }}
    />
  );
}

function JoinForm(props: {
  code: string;
  connecting: boolean;
  error: string | null;
  onJoin: (name: string) => void;
}) {
  const [name, setName] = useState(rememberedName);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (name.trim()) props.onJoin(name);
  };

  return (
    <form className="stack" onSubmit={submit}>
      <h1>{fr.join.title(props.code)}</h1>
      <label htmlFor="name">{fr.join.nameLabel}</label>
      <input
        id="name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder={fr.join.namePlaceholder}
        autoComplete="given-name"
        maxLength={20}
        autoFocus
      />
      <button className="primary" type="submit" disabled={!name.trim() || props.connecting}>
        {props.connecting ? fr.join.connecting : fr.join.submit}
      </button>
      {props.error && <p className="error">{props.error}</p>}
    </form>
  );
}

function Lobby(props: {
  view: PlayerView;
  call: Call;
  reconnecting: boolean;
  onLeave: () => void;
}) {
  const { view } = props;
  const url = lobbyUrl(view.code);
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
    } catch {
      // Clipboard refused (insecure context): the link stays visible to copy by hand.
    }
  };

  const share = () => navigator.share({ title: fr.lobby.shareTitle, url }).catch(() => {});

  const { call } = props;

  return (
    <div className="lobby">
      <section className="call stack">
        {props.reconnecting && <p className="banner">{fr.join.connectionLost}</p>}
        <CallNotices call={call} />
        <h2>{fr.lobby.players(view.players.length)}</h2>
        <CallGrid view={view} member={call.member} />
        {call.voices().map((track) => (
          <Voice key={track.sid} track={track} />
        ))}
      </section>

      <aside className="stack">
        <section className="card invite">
          <p className="muted">{fr.lobby.code}</p>
          <p className="code">{view.code}</p>
          <p className="muted">{fr.lobby.shareHint}</p>
          <p className="link">{url}</p>
          <div className="row">
            <button onClick={copy}>{copied ? fr.lobby.copied : fr.lobby.copy}</button>
            {"share" in navigator && <button onClick={share}>{fr.lobby.share}</button>}
          </div>
        </section>
        <button className="quiet" onClick={props.onLeave}>
          {fr.lobby.leave}
        </button>
      </aside>
    </div>
  );
}

/** What stands between this Player and a working call, with the way out. */
function CallNotices({ call }: { call: Call }) {
  if (call.status === "failed") {
    return (
      <div className="notice">
        <p>{fr.call.failed}</p>
        <button onClick={call.retryConnect}>{fr.call.retry}</button>
      </div>
    );
  }
  if (call.mic !== "pending" && call.mic !== "on") {
    return (
      <div className="notice" role="alert">
        <p>
          <strong>{fr.call.micRequired}</strong> {fr.call.micHelp[call.mic]}
        </p>
        <button onClick={call.retryMic}>{fr.call.retry}</button>
      </div>
    );
  }
  if (call.audioBlocked) {
    return (
      <button className="primary" onClick={call.startAudio}>
        {fr.call.enableSound}
      </button>
    );
  }
  return null;
}
