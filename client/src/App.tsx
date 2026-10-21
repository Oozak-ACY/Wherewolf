import { useEffect, useState, type FormEvent } from "react";
import { ConnectionLost, Paused } from "./Banners";
import { GamePanel } from "./GamePanel";
import { joinLink, lobbyCodeFromPath } from "./joinLink";
import type { MomentId } from "./generated/MomentId";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerView } from "./generated/PlayerView";
import type { Rejection } from "./generated/Rejection";
import type { Settings } from "./generated/Settings";
import { speaksAloud, unlockSpeech } from "./narrator";
import { NarratorBar } from "./NarratorBar";
import { narratorBar } from "./narratorBar";
import { PlayerGrid } from "./PlayerGrid";
import { QrCodeView } from "./QrCodeView";
import { cardTurnsFaceUp, RoleCardView } from "./RoleCardView";
import { rememberedName } from "./seat";
import { createLobby } from "./server";
import { SettingsPanel } from "./SettingsPanel";
import { ground } from "./ground";
import { pausedBy, sleepingScreen, waitingScreen } from "./sleepingScreen";
import { SleepingView } from "./SleepingView";
import { fr } from "./strings";
import { preloadTableAudio, unlockTableAudio } from "./tableAudio";
import { useLobby } from "./useLobby";
import { useNarrator } from "./useNarrator";
import { useGround } from "./useGround";
import { useWakeLock } from "./useWakeLock";

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
  const code = lobbyCodeFromPath(path);
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
        endsAt={lobby.endsAt}
        seatings={lobby.seatings}
        reconnecting={lobby.status === "reconnecting"}
        rejection={lobby.rejection}
        onSettingsChange={lobby.updateSettings}
        onStart={() => {
          // The Host's tap: the only moment iOS lets the Table device's audio
          // unlock for the whole Game.
          unlockTableAudio();
          preloadTableAudio();
          lobby.start();
        }}
        onConfirmRole={lobby.confirmRole}
        onEndDebate={lobby.endDebate}
        onChooseLovers={lobby.chooseLovers}
        onConfirmPartner={lobby.confirmPartner}
        onRob={lobby.rob}
        onInspect={lobby.inspect}
        onPickVictim={lobby.pickVictim}
        onHeal={lobby.heal}
        onPass={lobby.pass}
        onPoison={lobby.poison}
        onVote={lobby.vote}
        onShoot={lobby.shoot}
        onElect={lobby.elect}
        onBreakTie={lobby.breakTie}
        onNameSuccessor={lobby.nameSuccessor}
        onPlayAgain={() => {
          unlockTableAudio();
          lobby.playAgain();
        }}
        onAnnounced={lobby.announced}
        onLeave={() => {
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
        // Still inside the "Rejoindre" tap: a Host coming back mid-Game gets
        // no Start tap, and speech is their fallback voice.
        unlockSpeech();
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
  endsAt: number | null;
  seatings: number;
  reconnecting: boolean;
  rejection: Rejection | null;
  onSettingsChange: (settings: Settings) => void;
  onStart: () => void;
  onConfirmRole: () => void;
  onEndDebate: () => void;
  onChooseLovers: (lovers: PlayerId[]) => void;
  onConfirmPartner: () => void;
  onRob: (player: PlayerId) => void;
  onInspect: (player: PlayerId) => void;
  onPickVictim: (victim: PlayerId) => void;
  onHeal: () => void;
  onPass: () => void;
  onPoison: (player: PlayerId) => void;
  onVote: (designated: PlayerId | null) => void;
  onShoot: (player: PlayerId) => void;
  onElect: (candidate: PlayerId) => void;
  onBreakTie: (player: PlayerId) => void;
  onNameSuccessor: (player: PlayerId) => void;
  onPlayAgain: () => void;
  /** The Table device has said every line announcing this Moment. */
  onAnnounced: (moment: MomentId) => void;
  onLeave: () => void;
}) {
  const { view } = props;
  const held = view.role?.role ?? null;
  const started = held !== null;
  // The card turns face up by itself when the Roles are dealt, and when the Theft changes it.
  const [cardOpen, setCardOpen] = useState(started);
  const [shownRole, setShownRole] = useState(held);
  if (held !== shownRole) {
    setShownRole(held);
    if (cardTurnsFaceUp(shownRole, held)) setCardOpen(true);
  }
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

  // Silent while this phone has lost the server: for the Table device, the
  // Game is Paused, and it repeats the current line once it is back.
  useNarrator(view, view.code, speaksAloud(view) && !props.reconnecting, props.seatings, props.onAnnounced);
  useWakeLock(view.moment !== null);
  useGround(ground(view, cardOpen));

  // Asleep at Night, a phone shows only what every sleeping phone shows.
  // While the Narrator announces the coming Moment, every phone waits for the Narrator.
  const asleep = sleepingScreen(view);
  const waiting = waitingScreen(view);
  const paused = pausedBy(view);
  const content = asleep ? (
    <SleepingView screen={asleep} night reconnecting={props.reconnecting} />
  ) : waiting ? (
    <SleepingView screen={waiting} night={false} reconnecting={props.reconnecting} />
  ) : (
    <div className="lobby">
      <section className="stack">
        {props.reconnecting && <ConnectionLost />}
        {paused !== null && <Paused hostName={paused} />}
        {props.rejection && <p className="error">{fr.rejection[props.rejection]}</p>}
        <h2>{fr.lobby.players(view.players.length)}</h2>
        <PlayerGrid view={view} />
      </section>

      <aside className="stack">
        {view.role && cardOpen && (
          <RoleCardView
            card={view.role}
            view={view}
            onConfirm={props.onConfirmRole}
            onClose={() => setCardOpen(false)}
          />
        )}
        {view.moment ? (
          <GamePanel
            view={view}
            moment={view.moment}
            onConfirmRole={props.onConfirmRole}
            onEndDebate={props.onEndDebate}
            onChooseLovers={props.onChooseLovers}
            onConfirmPartner={props.onConfirmPartner}
            onRob={props.onRob}
            onInspect={props.onInspect}
            onPickVictim={props.onPickVictim}
            onHeal={props.onHeal}
            onPass={props.onPass}
            onPoison={props.onPoison}
            onVote={props.onVote}
            onShoot={props.onShoot}
            onElect={props.onElect}
            onBreakTie={props.onBreakTie}
            onNameSuccessor={props.onNameSuccessor}
            onPlayAgain={props.onPlayAgain}
          />
        ) : (
          <SettingsPanel view={view} onChange={props.onSettingsChange} onStart={props.onStart} />
        )}
        {!started && (
          <section className="card invite">
            <p className="muted">{fr.lobby.code}</p>
            <p className="code">{view.code}</p>
            <p className="muted">{fr.lobby.shareHint}</p>
            <p className="link">{url}</p>
            <div className="row">
              <button onClick={copy}>{copied ? fr.lobby.copied : fr.lobby.copy}</button>
              {"share" in navigator && <button onClick={share}>{fr.lobby.share}</button>}
            </div>
            {isHost && <button onClick={() => setQrOpen(true)}>{fr.lobby.showQrCode}</button>}
          </section>
        )}
        {isHost && !started && qrOpen && <QrCodeView link={url} onClose={() => setQrOpen(false)} />}
        <button className="quiet" onClick={props.onLeave}>
          {fr.lobby.leave}
        </button>
      </aside>
    </div>
  );

  const bar = narratorBar(view);
  return (
    <>
      {/* First, so the mixer stays open from one screen to the next. */}
      {bar && (
        <NarratorBar bar={bar} endsAt={props.endsAt} paused={view.paused} onOpenCard={() => setCardOpen(true)} />
      )}
      {content}
    </>
  );
}
