import { Choice, Footer } from "./Choice";
import type { Ballot } from "./generated/Ballot";
import type { Moment } from "./generated/Moment";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerView } from "./generated/PlayerView";
import { picking } from "./ground";
import { partnerOf } from "./RoleCardView";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
  onConfirmRole: () => void;
  onEndDebate: () => void;
  onPass: () => void;
  onVote: (designated: PlayerId | null) => void;
  onShoot: (player: PlayerId) => void;
  onElect: (candidate: PlayerId) => void;
  onBreakTie: (player: PlayerId) => void;
  onNameSuccessor: (player: PlayerId) => void;
  onPlayAgain: () => void;
};

/**
 * The Game as this Player may see it by Day, under the Narrator bar: what
 * they can do. Night Turns have their own screen.
 */
export function GamePanel(props: Props) {
  const { view, moment } = props;
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const living = view.players.filter((p) => p.alive);
  const mode = picking(view);

  return (
    <section className="card stack game">
      <GameNotes view={view} moment={moment} />

      {moment.type === "roleReveal" && (
        <>
          <p className="muted">{fr.game.stillReading(moment.stillReading)}</p>
          {!view.spectating &&
            (moment.confirmed ? (
              <p className="muted">{fr.game.roleConfirmed}</p>
            ) : (
              <Footer>
                <button className="primary" onClick={props.onConfirmRole}>
                  {fr.game.confirmRole}
                </button>
              </Footer>
            ))}
        </>
      )}

      {moment.type === "discussion" && view.mayEndDebate && (
        <Footer>
          <button className="primary" onClick={props.onEndDebate}>
            {view.mayor === null ? fr.game.endDebateToElection : fr.game.endDebateToVote}
          </button>
        </Footer>
      )}

      {moment.type === "election" && (
        <>
          <p className="muted">{fr.game.votedCount(moment.voted.length, living.length)}</p>
          {mode && (
            <>
              <p>{fr.game.electPrompt}</p>
              {moment.yourBallot?.designated != null && (
                <p className="muted">{fr.game.yourVote(name(moment.yourBallot.designated))}</p>
              )}
              <Choice
                mode={mode}
                players={living}
                chosen={moment.yourBallot?.designated ?? null}
                onChoose={props.onElect}
              />
            </>
          )}
        </>
      )}

      {moment.type === "electionResult" && <BallotList ballots={moment.ballots} name={name} />}

      {moment.type === "vote" && (
        <>
          <p className="muted">{fr.game.votedCount(moment.voted.length, living.length)}</p>
          {mode && (
            <>
              <p>{fr.game.votePrompt}</p>
              {moment.yourBallot && (
                <p className="muted">
                  {moment.yourBallot.designated === null
                    ? fr.game.youAbstained
                    : fr.game.yourVote(name(moment.yourBallot.designated))}
                </p>
              )}
              <Choice
                mode={mode}
                players={living}
                chosen={moment.yourBallot?.designated ?? null}
                onChoose={props.onVote}
              >
                <button
                  className={moment.yourBallot && moment.yourBallot.designated === null ? "quiet chosen" : "quiet"}
                  aria-pressed={moment.yourBallot?.designated === null}
                  onClick={() => props.onVote(null)}
                >
                  {fr.game.abstain}
                </button>
              </Choice>
            </>
          )}
        </>
      )}

      {moment.type === "tieBreak" && (
        <>
          <BallotList ballots={moment.ballots} name={name} />
          {mode ? (
            <>
              <p>{fr.game.tieBreakPick}</p>
              <Choice
                mode={mode}
                players={living.filter((p) => moment.tied.includes(p.id))}
                detail={(id) => {
                  const by = votersFor(moment.ballots, id);
                  return by.length > 0 ? fr.game.pickedBy(by.map(name)) : null;
                }}
                count={(id) => votersFor(moment.ballots, id).length}
                confirmLabel={fr.game.confirm.breakTie}
                onChoose={props.onBreakTie}
              >
                <button className="quiet" onClick={props.onPass}>
                  {fr.game.tieBreakPass}
                </button>
              </Choice>
            </>
          ) : (
            view.mayor !== null && <p className="muted">{fr.game.mayorChoosing(name(view.mayor))}</p>
          )}
        </>
      )}

      {moment.type === "voteResult" && <BallotList ballots={moment.ballots} name={name} />}

      {moment.type === "huntersShot" &&
        (mode ? (
          <>
            <p>{fr.game.shootPick}</p>
            <Choice
              mode={mode}
              players={living}
              confirmLabel={fr.game.confirm.shoot}
              onChoose={props.onShoot}
            >
              <button className="quiet" onClick={props.onPass}>
                {fr.game.shootPass}
              </button>
            </Choice>
          </>
        ) : (
          <p className="muted">{fr.game.hunterAiming(name(moment.hunter))}</p>
        ))}

      {moment.type === "succession" &&
        (mode ? (
          <>
            <p>{fr.game.successorPick}</p>
            <Choice
              mode={mode}
              players={living}
              confirmLabel={fr.game.confirm.nameSuccessor}
              onChoose={props.onNameSuccessor}
            />
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
          <Footer>
            <button className="primary" onClick={props.onPlayAgain}>
              {fr.game.playAgain}
            </button>
          </Footer>
        ) : (
          <p className="muted">{fr.game.waitingForHostToPlayAgain}</p>
        ))}
    </section>
  );
}

/** Who voted for `designated`. */
function votersFor(ballots: Ballot[], designated: PlayerId): PlayerId[] {
  return ballots.filter((b) => b.designated === designated).map((b) => b.voter);
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

/** What a Player knows beyond their own Role: that they are a Spectator, the Lovers, the Theft. */
export function GameNotes({ view, moment }: { view: PlayerView; moment: Moment }) {
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const partner = partnerOf(view);
  return (
    <>
      {view.spectating && moment.type !== "victory" && <p className="muted">{fr.game.spectating}</p>}
      {/* A Spectator knows the Lovers from the moment Cupid names them, and
          everyone does on the victory screen. */}
      {(partner === undefined || moment.type === "victory") && view.lovers.length === 2 && (
        <p>{fr.game.lovers(name(view.lovers[0]!), name(view.lovers[1]!))}</p>
      )}
      {/* Only a Spectator, or everyone on the victory screen, knows who the Thief robbed. */}
      {view.theft && (
        <p>{fr.game.theft(name(view.theft.thief), view.theft.robbed === null ? null : name(view.theft.robbed))}</p>
      )}
    </>
  );
}
