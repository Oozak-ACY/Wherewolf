import { useEffect, useState } from "react";
import type { Ballot } from "./generated/Ballot";
import type { Moment } from "./generated/Moment";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { narratorLine } from "./narrator";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
  endsAt: number | null;
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

/** The Game as this Player may see it: the Narrator's line, then what they can do. */
export function GamePanel(props: Props) {
  const { view, moment } = props;
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const me = view.players.find((p) => p.id === view.you);
  const living = view.players.filter((p) => p.alive);

  return (
    <section className="card stack game">
      <p className="narrator" aria-live="polite">
        {narratorLine(moment, view.players, view.settings)}
      </p>
      {props.endsAt !== null && <Countdown endsAt={props.endsAt} />}
      {view.spectating && moment.type !== "victory" && <p className="muted">{fr.game.spectating}</p>}

      {moment.type === "seersTurn" &&
        (moment.inspection ? (
          <p>{fr.game.seerSaw(name(moment.inspection.player), fr.roles[moment.inspection.role].name)}</p>
        ) : view.role?.role === "seer" && me?.alive ? (
          <>
            <p>{fr.game.seerPick}</p>
            <ChoiceList
              players={living.filter((p) => p.id !== view.you)}
              chosen={null}
              onChoose={props.onInspect}
            />
          </>
        ) : me?.alive ? (
          <p className="muted">{fr.game.asleep}</p>
        ) : (
          <p className="muted">{fr.game.seerSawNothing}</p>
        ))}

      {moment.type === "werewolvesTurn" &&
        (moment.picks && me?.alive ? (
          <>
            <p>{fr.game.werewolvesPick}</p>
            <p className="muted">{fr.game.unanimity}</p>
            <ChoiceList
              players={living}
              chosen={moment.picks.find((p) => p.werewolf === view.you)?.victim ?? null}
              detail={(id) => {
                const by = moment.picks!.filter((p) => p.victim === id).map((p) => name(p.werewolf));
                return by.length > 0 ? fr.game.pickedBy(by) : null;
              }}
              onChoose={props.onPickVictim}
            />
          </>
        ) : moment.picks ? (
          <ul className="ballots">
            {moment.picks.map((p) => (
              <li key={p.werewolf}>{fr.game.ballot(name(p.werewolf), name(p.victim))}</li>
            ))}
          </ul>
        ) : (
          <p className="muted">{fr.game.asleep}</p>
        ))}

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
        ) : (
          <p className="muted">{fr.game.asleep}</p>
        ))}

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

/** Time left until `endsAt`, as m:ss. */
function Countdown({ endsAt }: { endsAt: number }) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const tick = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(tick);
  }, []);
  const seconds = Math.max(0, Math.ceil((endsAt - now) / 1000));
  const text = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  return (
    <p className="countdown" role="timer">
      {text}
    </p>
  );
}
