import { useEffect, useState } from "react";
import type { Moment } from "./generated/Moment";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
  endsAt: number | null;
  onPickVictim: (victim: PlayerId) => void;
  onVote: (designated: PlayerId | null) => void;
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
        {narratorLine(moment, view.players, name)}
      </p>
      {props.endsAt !== null && <Countdown endsAt={props.endsAt} />}
      {me && !me.alive && moment.type !== "victory" && <p className="muted">{fr.game.spectating}</p>}

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
        ) : (
          me?.alive && <p className="muted">{fr.game.asleep}</p>
        ))}

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

      {moment.type === "voteResult" && moment.ballots.length > 0 && (
        <ul className="ballots">
          {moment.ballots.map((b) => (
            <li key={b.voter}>
              {fr.game.ballot(name(b.voter), b.designated === null ? null : name(b.designated))}
            </li>
          ))}
        </ul>
      )}

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

function narratorLine(
  moment: Moment,
  players: PlayerSummary[],
  name: (id: PlayerId) => string,
): string {
  const roleOf = (id: PlayerId) => {
    const role = players.find((p) => p.id === id)?.revealedRole;
    return role ? fr.roles[role].name : "?";
  };
  switch (moment.type) {
    case "werewolvesTurn":
      return fr.narrator.werewolvesTurn;
    case "dawn":
      return moment.deaths.length === 0
        ? fr.narrator.dawnNobody
        : fr.narrator.dawnDeaths(
            moment.deaths.map((id) => `${name(id)} (${roleOf(id)})`).join(", "),
          );
    case "discussion":
      return fr.narrator.discussion;
    case "vote":
      return fr.narrator.vote;
    case "voteResult":
      return moment.eliminated === null
        ? fr.narrator.voteNobody
        : fr.narrator.voteEliminated(name(moment.eliminated), roleOf(moment.eliminated));
    case "victory":
      return fr.narrator.victory[moment.winner];
  }
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
