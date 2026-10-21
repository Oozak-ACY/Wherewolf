import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { Alert, ConnectionLost, Paused } from "./Banners";
import { GamePanel } from "./GamePanel";
import { lobbyCodeFromPath } from "./joinLink";
import { LobbyView } from "./LobbyView";
import type { MomentId } from "./generated/MomentId";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerView } from "./generated/PlayerView";
import type { Rejection } from "./generated/Rejection";
import type { Settings } from "./generated/Settings";
import { speaksAloud, unlockSpeech } from "./narrator";
import { NarratorBar } from "./NarratorBar";
import { narratorBar } from "./narratorBar";
import { PlayerGrid } from "./PlayerGrid";
import { cardImageUrl, cardTurnsFaceUp, RoleCardView } from "./RoleCardView";
import { rememberedName } from "./seat";
import { createLobby } from "./server";
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
    <Frame>
      <p className="tagline">{fr.tagline}</p>
      <div className="frame-actions">
        <button className="primary" onClick={create} disabled={creating} aria-busy={creating}>
          {creating ? fr.home.creating : fr.home.create}
        </button>
        {failed && <Alert>{fr.home.createFailed}</Alert>}
        <p className="divider">{fr.home.or}</p>
        <form className="stack" onSubmit={goTo}>
          <label htmlFor="code">{fr.home.joinWithCode}</label>
          <div className="code-row">
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
      </div>
    </Frame>
  );
}

/** The double frame of the screens before joining a Lobby, under the wordmark and the cards */
function Frame({ children }: { children: ReactNode }) {
  return (
    <div className="frame">
      <header className="wordmark">
        <p>{fr.kicker}</p>
        <h1>{fr.appName}</h1>
      </header>
      <CardFan />
      {children}
    </div>
  );
}

/** The Seer and the Villager fanned behind the Werewolf */
function CardFan() {
  return (
    <div className="card-fan" aria-hidden="true">
      <img className="card-art" src={cardImageUrl("seer")} alt="" />
      <img className="card-art" src={cardImageUrl("villager")} alt="" />
      <img className="card-art" src={cardImageUrl("werewolf")} alt="" />
    </div>
  );
}

function LobbyScreen({ code, onExit }: { code: string; onExit: () => void }) {
  const lobby = useLobby(code);

  if (lobby.status === "notFound") {
    return (
      <Frame>
        <div className="frame-actions">
          <Alert>{fr.join.lobbyNotFound}</Alert>
          <button className="primary" onClick={onExit}>
            {fr.join.backHome}
          </button>
        </div>
      </Frame>
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
    <Frame>
      <h2 className="frame-title">{fr.join.title(props.code)}</h2>
      <form className="stack frame-actions" onSubmit={submit}>
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
        <button className="primary" type="submit" disabled={!name.trim() || props.connecting} aria-busy={props.connecting}>
          {props.connecting ? fr.join.connecting : fr.join.submit}
        </button>
        {props.error && <Alert>{props.error}</Alert>}
      </form>
    </Frame>
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
  const banners = (
    <>
      {props.reconnecting && <ConnectionLost />}
      {paused !== null && <Paused hostName={paused} />}
      {props.rejection && <p className="error">{fr.rejection[props.rejection]}</p>}
    </>
  );
  const content = asleep ? (
    <SleepingView screen={asleep} night reconnecting={props.reconnecting} />
  ) : waiting ? (
    <SleepingView screen={waiting} night={false} reconnecting={props.reconnecting} />
  ) : !view.moment ? (
    <LobbyView
      view={view}
      banners={banners}
      onSettingsChange={props.onSettingsChange}
      onStart={props.onStart}
      onLeave={props.onLeave}
    />
  ) : (
    <div className="lobby">
      <section className="stack">
        {banners}
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
