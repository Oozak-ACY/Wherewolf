import { useEffect, useState } from "react";
import type { Ballot } from "./generated/Ballot";
import type { Moment } from "./generated/Moment";
import type { Pick } from "./generated/Pick";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { narratorText } from "./narratorScript";
import { partnerOf } from "./RoleCardView";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
  endsAt: number | null;
  onConfirmRole: () => void;
  onEndDebate: () => void;
  onChooseLovers: (lovers: PlayerId[]) => void;
  onConfirmPartner: () => void;
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
};

/**
 * The Game as this Player may see it: the Narrator's line, then what they can
 * do. At Night, only the acting Role and the Spectators get here; every other
 * phone shows the sleeping screen instead.
 */
export function GamePanel(props: Props) {
  const { view, moment } = props;
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const me = view.players.find((p) => p.id === view.you);
  const living = view.players.filter((p) => p.alive);
  const partner = partnerOf(view);

  return (
    <section className="card stack game">
      <p className="narrator" aria-live="polite">
        {narratorText(view)}
      </p>
      {view.paused && <p className="banner">{fr.game.paused(name(view.host))}</p>}
      {props.endsAt !== null && <Countdown endsAt={props.endsAt} paused={view.paused} />}
      {view.spectating && moment.type !== "victory" && <p className="muted">{fr.game.spectating}</p>}
      {/* A Spectator knows the Lovers from the moment Cupid names them, and
          everyone does on the victory screen. */}
      {(partner === undefined || moment.type === "victory") && view.lovers.length === 2 && (
        <p>{fr.game.lovers(name(view.lovers[0]!), name(view.lovers[1]!))}</p>
      )}

      {moment.type === "roleReveal" && (
        <>
          <p className="muted">{fr.game.stillReading(moment.stillReading)}</p>
          {!view.spectating &&
            (moment.confirmed ? (
              <p className="muted">{fr.game.roleConfirmed}</p>
            ) : (
              <button className="primary" onClick={props.onConfirmRole}>
                {fr.game.confirmRole}
              </button>
            ))}
        </>
      )}

      {moment.type === "discussion" && view.mayEndDebate && (
        <button className="primary" onClick={props.onEndDebate}>
          {view.mayor === null ? fr.game.endDebateToElection : fr.game.endDebateToVote}
        </button>
      )}

      {moment.type === "cupidsTurn" &&
        (view.role?.role === "cupid" && me?.alive ? (
          <CupidsPick living={living} onChooseLovers={props.onChooseLovers} />
        ) : (
          <p className="muted">{fr.game.cupidChoosing}</p>
        ))}

      {moment.type === "loversTurn" &&
        (partner !== undefined && me?.alive ? (
          <>
            <p>{fr.game.partner(name(partner))}</p>
            {moment.confirmed ? (
              <p className="muted">{fr.game.partnerConfirmed}</p>
            ) : (
              <button className="primary" onClick={props.onConfirmPartner}>
                {fr.game.partnerConfirm}
              </button>
            )}
          </>
        ) : null)}

      {moment.type === "seersTurn" &&
        (moment.inspection ? (
          <p>{fr.game.seerSaw(name(moment.inspection.player), fr.roles[moment.inspection.role].name)}</p>
        ) : moment.passed ? (
          <p className="muted">{fr.game.seerPassed}</p>
        ) : view.role?.role === "seer" && me?.alive ? (
          <>
            <p>{fr.game.seerPick}</p>
            <ChoiceList
              players={living.filter((p) => p.id !== view.you)}
              chosen={null}
              onChoose={props.onInspect}
            />
            <button onClick={props.onPass}>{fr.game.seerPass}</button>
          </>
        ) : (
          <p className="muted">{fr.game.seerSawNothing}</p>
        ))}

      {moment.type === "werewolvesTurn" &&
        (moment.picks && me?.alive ? (
          <WerewolvesPick
            picks={moment.picks}
            you={view.you}
            living={living}
            name={name}
            onPickVictim={props.onPickVictim}
            onPass={props.onPass}
          />
        ) : moment.picks ? (
          <ul className="ballots">
            {moment.picks.map((p) => (
              <li key={p.werewolf}>
                {fr.game.pick(name(p.werewolf), p.victim === null ? null : name(p.victim))}
              </li>
            ))}
          </ul>
        ) : null)}

      {moment.type === "witchsTurn" &&
        (moment.witch ? (
          <>
            <p>
              {moment.witch.victim === null
                ? fr.game.witchNoVictim
                : fr.game.witchVictim(name(moment.witch.victim))}
            </p>
            {moment.witch.healed && moment.witch.victim !== null && (
              <p>{fr.game.witchHealed(name(moment.witch.victim))}</p>
            )}
            {moment.witch.poisoned !== null && <p>{fr.game.witchPoisoned(name(moment.witch.poisoned))}</p>}
            {moment.witch.passed && <p className="muted">{fr.game.witchPassed}</p>}
            {me?.alive && view.role?.potions && !moment.witch.passed && (
              <>
                <p className="muted">
                  {fr.game.potionsLeft(view.role.potions.healing, view.role.potions.poison)}
                </p>
                {view.role.potions.healing && moment.witch.victim !== null && (
                  <button onClick={props.onHeal}>{fr.game.heal}</button>
                )}
                {view.role.potions.poison && (
                  <>
                    <p>{fr.game.poisonPick}</p>
                    <ChoiceList players={living} chosen={null} onChoose={props.onPoison} />
                  </>
                )}
                <button onClick={props.onPass}>{fr.game.pass}</button>
              </>
            )}
          </>
        ) : null)}

      {moment.type === "election" && (
        <>
          <p className="muted">{fr.game.votedCount(moment.voted.length, living.length)}</p>
          {me?.alive && (
            <>
              <p>{fr.game.electPrompt}</p>
              <ChoiceList
                players={living}
                chosen={moment.yourBallot?.designated ?? null}
                onChoose={props.onElect}
              />
              {moment.yourBallot?.designated != null && (
                <p className="muted">{fr.game.yourVote(name(moment.yourBallot.designated))}</p>
              )}
            </>
          )}
        </>
      )}

      {moment.type === "electionResult" && <BallotList ballots={moment.ballots} name={name} />}

      {moment.type === "vote" && (
        <>
          <p className="muted">{fr.game.votedCount(moment.voted.length, living.length)}</p>
          {me?.alive && (
            <>
              <p>{fr.game.votePrompt}</p>
              <ChoiceList
                players={living}
                chosen={moment.yourBallot?.designated ?? null}
                onChoose={props.onVote}
              />
              <button
                className={moment.yourBallot && moment.yourBallot.designated === null ? "chosen" : ""}
                onClick={() => props.onVote(null)}
              >
                {fr.game.abstain}
              </button>
              {moment.yourBallot && (
                <p className="muted">
                  {moment.yourBallot.designated === null
                    ? fr.game.youAbstained
                    : fr.game.yourVote(name(moment.yourBallot.designated))}
                </p>
              )}
            </>
          )}
        </>
      )}

      {moment.type === "tieBreak" && (
        <>
          <BallotList ballots={moment.ballots} name={name} />
          {view.mayor === view.you && me?.alive ? (
            <>
              <p>{fr.game.tieBreakPick}</p>
              <ChoiceList
                players={living.filter((p) => moment.tied.includes(p.id))}
                chosen={null}
                onChoose={props.onBreakTie}
              />
              <button onClick={props.onPass}>{fr.game.tieBreakPass}</button>
            </>
          ) : (
            view.mayor !== null && <p className="muted">{fr.game.mayorChoosing(name(view.mayor))}</p>
          )}
        </>
      )}

      {moment.type === "voteResult" && <BallotList ballots={moment.ballots} name={name} />}

      {moment.type === "huntersShot" &&
        (moment.hunter === view.you ? (
          <>
            <p>{fr.game.shootPick}</p>
            <ChoiceList players={living} chosen={null} onChoose={props.onShoot} />
            <button onClick={props.onPass}>{fr.game.shootPass}</button>
          </>
        ) : (
          <p className="muted">{fr.game.hunterAiming(name(moment.hunter))}</p>
        ))}

      {moment.type === "succession" &&
        (moment.mayor === view.you ? (
          <>
            <p>{fr.game.successorPick}</p>
            <ChoiceList players={living} chosen={null} onChoose={props.onNameSuccessor} />
          </>
        ) : (
          <p className="muted">{fr.game.mayorNaming(name(moment.mayor))}</p>
        ))}

      {moment.type === "victory" && (
        <>
          <h3>{fr.game.everyRole}</h3>
          <ul className="ballots">
            {view.players.map((p) => (
              <li key={p.id}>
                {p.name} : {p.revealedRole ? fr.roles[p.revealedRole].name : "?"}
              </li>
            ))}
          </ul>
        </>
      )}

      {moment.type === "victory" &&
        (view.you === view.host ? (
          <button className="primary" onClick={props.onPlayAgain}>
            {fr.game.playAgain}
          </button>
        ) : (
          <p className="muted">{fr.game.waitingForHostToPlayAgain}</p>
        ))}
    </section>
  );
}

/** Cupid's choice of the two Lovers, any two living Players, sent once both are chosen. */
function CupidsPick(props: { living: PlayerSummary[]; onChooseLovers: (lovers: PlayerId[]) => void }) {
  const [chosen, setChosen] = useState<PlayerId[]>([]);
  // A third choice replaces the oldest one.
  const toggle = (id: PlayerId) =>
    setChosen((now) => (now.includes(id) ? now.filter((c) => c !== id) : [...now, id].slice(-2)));
  return (
    <>
      <p>{fr.game.cupidPick}</p>
      <ul className="choices">
        {props.living.map((p) => (
          <li key={p.id}>
            <button
              className={chosen.includes(p.id) ? "chosen" : ""}
              aria-pressed={chosen.includes(p.id)}
              onClick={() => toggle(p.id)}
            >
              {p.name}
            </button>
          </li>
        ))}
      </ul>
      <button className="primary" disabled={chosen.length !== 2} onClick={() => props.onChooseLovers(chosen)}>
        {fr.game.cupidConfirm}
      </button>
    </>
  );
}

/** A living Werewolf's choice: a Victim, or nobody. Every Werewolf sees the others' picks. */
function WerewolvesPick(props: {
  picks: Pick[];
  you: PlayerId;
  living: PlayerSummary[];
  name: (id: PlayerId) => string;
  onPickVictim: (victim: PlayerId) => void;
  onPass: () => void;
}) {
  const { picks, name } = props;
  const mine = picks.find((p) => p.werewolf === props.you);
  const pickedBy = (victim: PlayerId | null) =>
    picks.filter((p) => p.victim === victim).map((p) => name(p.werewolf));
  const passedBy = pickedBy(null);
  return (
    <>
      <p>{fr.game.werewolvesPick}</p>
      <p className="muted">{fr.game.unanimity}</p>
      <ChoiceList
        players={props.living}
        chosen={mine?.victim ?? null}
        detail={(id) => {
          const by = pickedBy(id);
          return by.length > 0 ? fr.game.pickedBy(by) : null;
        }}
        onChoose={props.onPickVictim}
      />
      <button
        className={mine && mine.victim === null ? "chosen" : ""}
        aria-pressed={mine?.victim === null}
        onClick={props.onPass}
      >
        <span>{fr.game.werewolvesPass}</span>
        {passedBy.length > 0 && <span className="muted"> {fr.game.pickedBy(passedBy)}</span>}
      </button>
    </>
  );
}

/** Who voted for whom, once revealed. Nothing if nobody voted. */
function BallotList(props: { ballots: Ballot[]; name: (id: PlayerId) => string }) {
  if (props.ballots.length === 0) return null;
  return (
    <ul className="ballots">
      {props.ballots.map((b) => (
        <li key={b.voter}>
          {fr.game.ballot(props.name(b.voter), b.designated === null ? null : props.name(b.designated))}
        </li>
      ))}
    </ul>
  );
}

/** One button per living Player; the current choice stands out. */
function ChoiceList(props: {
  players: PlayerSummary[];
  chosen: PlayerId | null;
  detail?: (id: PlayerId) => string | null;
  onChoose: (id: PlayerId) => void;
}) {
  return (
    <ul className="choices">
      {props.players.map((p) => {
        const detail = props.detail?.(p.id);
        return (
          <li key={p.id}>
            <button
              className={p.id === props.chosen ? "chosen" : ""}
              aria-pressed={p.id === props.chosen}
              onClick={() => props.onChoose(p.id)}
            >
              <span>{p.name}</span>
              {detail && <span className="muted">{detail}</span>}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** Time left until `endsAt`, as m:ss. Still while the Game is Paused. */
function Countdown({ endsAt, paused }: { endsAt: number; paused: boolean }) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    if (paused) return;
    const tick = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(tick);
  }, [endsAt, paused]);
  const seconds = Math.max(0, Math.ceil((endsAt - now) / 1000));
  const text = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  return (
    <p className="countdown" role="timer">
      {text}
    </p>
  );
}
