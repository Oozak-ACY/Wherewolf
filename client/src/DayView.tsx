import type { ReactNode } from "react";
import { Choice, Footer, PickRows } from "./Choice";
import { abstainers, victoryTag, listTag, mayorNote, nameList, nameOf, outcome, votersFor, yourVote } from "./day";
import type { Ballot } from "./generated/Ballot";
import type { Moment } from "./generated/Moment";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { picking } from "./ground";
import { initial, isHost } from "./lobby";
import { cardImageUrl, partnerOf } from "./RoleCardView";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
  banners: ReactNode;
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
 * The Game by Day under the Narrator bar and public to the whole table
 * with the title of what the village decides and how it stands then the
 * pick rows or the Player list and the footer
 * Night Turns have their own screen
 */
export function DayView(props: Props) {
  const { view, moment } = props;
  const name = (id: PlayerId) => nameOf(view, id);
  const living = view.players.filter((p) => p.alive);
  const mode = picking(view);
  const players = <PlayerList view={view} />;
  const voted = (count: number) => fr.game.votedCount(count, living.length);
  const village = head(fr.game.villageTitle, status(null, fr.game.aliveCount(living.length, view.players.length)));

  let content: ReactNode;
  switch (moment.type) {
    case "roleReveal":
      content = (
        <>
          <p className="muted">{fr.game.stillReading(moment.stillReading)}</p>
          {!view.spectating && moment.confirmed && <p className="muted">{fr.game.roleConfirmed}</p>}
        </>
      );
      break;

    case "discussion":
      content = (
        <>
          {village}
          {players}
          {view.mayEndDebate && (
            <Footer>
              <button className="primary" onClick={props.onEndDebate}>
                {fr.game.endDebateToVote}
              </button>
            </Footer>
          )}
        </>
      );
      break;

    case "election":
      content = (
        <>
          {head(fr.game.electPrompt, status(yourVote(view), voted(moment.voted.length)))}
          {mode ? (
            <Choice mode={mode} players={living} chosen={moment.yourBallot?.designated ?? null} onChoose={props.onElect} />
          ) : (
            players
          )}
        </>
      );
      break;

    case "vote": {
      const abstained = moment.yourBallot !== null && moment.yourBallot.designated === null;
      content = (
        <>
          {head(fr.game.votePrompt, status(yourVote(view), voted(moment.voted.length)))}
          {mode ? (
            <Choice mode={mode} players={living} chosen={moment.yourBallot?.designated ?? null} onChoose={props.onVote}>
              <button
                className={abstained ? "quiet chosen" : "quiet"}
                aria-pressed={abstained}
                onClick={() => props.onVote(null)}
              >
                {fr.game.abstain}
              </button>
            </Choice>
          ) : (
            players
          )}
          <MayorNote view={view} />
        </>
      );
      break;
    }

    case "tieBreak": {
      const tied = living.filter((p) => moment.tied.includes(p.id));
      const tally = tallyOf(view, moment.ballots);
      content = (
        <>
          {head(
            fr.game.votePrompt,
            mode ? (
              <p>{fr.game.tieBreakPick}</p>
            ) : (
              view.mayor !== null && <p className="muted">{fr.game.mayorChoosing(name(view.mayor))}</p>
            ),
          )}
          {mode ? (
            <Choice
              mode={mode}
              players={tied}
              {...tally}
              confirmLabel={fr.game.confirm.breakTie}
              onChoose={props.onBreakTie}
              actions={[{ label: fr.game.tieBreakPass, onChoose: props.onPass }]}
            />
          ) : (
            <PickRows players={tied} {...tally} />
          )}
          <Abstentions view={view} ballots={moment.ballots} />
        </>
      );
      break;
    }

    case "electionResult":
    case "voteResult":
      content = (
        <>
          {head(moment.type === "electionResult" ? fr.game.electPrompt : fr.game.votePrompt)}
          <PickRows
            players={view.players.filter((p) => p.alive || votersFor(moment.ballots, p.id).length > 0)}
            {...tallyOf(view, moment.ballots)}
          />
          <Abstentions view={view} ballots={moment.ballots} />
        </>
      );
      break;

    case "huntersShot":
      content = (
        <>
          {head(
            fr.game.shotTitle,
            mode ? <p>{fr.game.shootPick}</p> : <p className="muted">{fr.game.hunterAiming(name(moment.hunter))}</p>,
          )}
          {mode ? (
            <Choice
              mode={mode}
              players={living}
              confirmLabel={fr.game.confirm.shoot}
              onChoose={props.onShoot}
              actions={[{ label: fr.game.shootPass, onChoose: props.onPass }]}
            />
          ) : (
            players
          )}
        </>
      );
      break;

    case "succession":
      content = (
        <>
          {head(
            fr.game.successionTitle,
            mode ? <p>{fr.game.successorPick}</p> : <p className="muted">{fr.game.mayorNaming(name(moment.mayor))}</p>,
          )}
          {mode ? (
            <Choice mode={mode} players={living} confirmLabel={fr.game.confirm.nameSuccessor} onChoose={props.onNameSuccessor} />
          ) : (
            players
          )}
        </>
      );
      break;

    case "victory": {
      const { title, pill } = outcome(moment.winner);
      content = (
        <>
          <header className="day-head victory-head">
            <h1>{title}</h1>
            {pill && (
              <p className="camp">
                <span className="eyebrow">{fr.game.winningCamp}</span>
                <span className="camp-pill">{pill}</span>
              </p>
            )}
          </header>
          <h2 className="victory-roles">{fr.game.everyRole}</h2>
          <ul className="choices">
            {view.players.map((p) => (
              <VictoryRow key={p.id} view={view} player={p} />
            ))}
          </ul>
          <Footer>
            {isHost(view) ? (
              <button className="primary" onClick={props.onPlayAgain}>
                {fr.game.playAgain}
              </button>
            ) : (
              <p className="muted">{fr.game.waitingForHostToPlayAgain}</p>
            )}
          </Footer>
        </>
      );
      break;
    }

    default:
      content = (
        <>
          {village}
          {players}
        </>
      );
  }

  return (
    <section className="day">
      {props.banners}
      {content}
      <GameNotes view={view} moment={moment} />
    </section>
  );
}

/** The title of what the village decides with how it stands under it */
function head(title: string, below?: ReactNode) {
  return (
    <header className="day-head">
      <h1>{title}</h1>
      {below}
    </header>
  );
}

/** This Player's own vote next to how many have voted */
function status(mine: string | null, count: string) {
  return (
    <div className="day-status">
      <span>{mine}</span>
      <span className="muted">{count}</span>
    </div>
  );
}

/** Who voted for each Player and how many once the ballots are known */
function tallyOf(view: PlayerView, ballots: Ballot[]) {
  return {
    detail: (id: PlayerId) => nameList(view, votersFor(ballots, id)),
    count: (id: PlayerId) => votersFor(ballots, id).length,
  };
}

function Abstentions({ view, ballots }: { view: PlayerView; ballots: Ballot[] }) {
  const names = nameList(view, abstainers(ballots));
  return names && <p className="muted">{fr.game.abstainedBy(names)}</p>;
}

/** Every Player read only with the dead greyed and the Mayor badged and each Role this Player may see */
export function PlayerList({ view }: { view: PlayerView }) {
  return (
    <ul className="choices">
      {view.players.map((p) => (
        <PlayerRow key={p.id} view={view} player={p} />
      ))}
    </ul>
  );
}

function PlayerRow({ view, player }: { view: PlayerView; player: PlayerSummary }) {
  const tag = listTag(view, player);
  return (
    <li className="list-row" data-dead={!player.alive}>
      <span className="avatar" aria-hidden="true">
        {initial(player.name)}
      </span>
      <span className="pick-name">
        <span>{player.name}</span>
        {tag && <span className="muted">{tag}</span>}
      </span>
      {player.id === view.mayor && (
        <span className="badge">
          <MayorCard />
          {fr.lobby.mayor}
        </span>
      )}
      {player.revealedRole && <img className="card-art mini" src={cardImageUrl(player.revealedRole)} alt="" />}
    </li>
  );
}

/** A Player on the victory screen with their Role card and name and Role and whether they survived */
function VictoryRow({ view, player }: { view: PlayerView; player: PlayerSummary }) {
  return (
    <li className="list-row victory-row" data-dead={!player.alive}>
      {player.revealedRole ? (
        <img className="card-art" src={cardImageUrl(player.revealedRole)} alt="" />
      ) : (
        <span className="avatar" aria-hidden="true">
          {initial(player.name)}
        </span>
      )}
      <span className="pick-name">
        <span>{player.name}</span>
        <span className="muted">{victoryTag(view, player)}</span>
      </span>
      {player.id === view.mayor && (
        <span className="badge">
          <MayorCard />
          {fr.lobby.mayor}
        </span>
      )}
    </li>
  );
}

/** The dashed note with the Mayor's card on how a tied Vote is settled */
function MayorNote({ view }: { view: PlayerView }) {
  const note = mayorNote(view);
  if (!note) return null;
  return (
    <p className="mayor-note">
      <MayorCard />
      <span>{note}</span>
    </p>
  );
}

function MayorCard() {
  return <img className="card-art" src={cardImageUrl("mayor")} alt="" />;
}

/** What a Player knows beyond their own Role such as being a Spectator or the Lovers or the Theft */
export function GameNotes({ view, moment }: { view: PlayerView; moment: Moment }) {
  const name = (id: PlayerId) => nameOf(view, id);
  const partner = partnerOf(view);
  return (
    <>
      {view.spectating && moment.type !== "victory" && <p className="muted">{fr.game.spectating}</p>}
      {/* A Spectator knows the Lovers from the moment Cupid names them and
          everyone does on the victory screen */}
      {(partner === undefined || moment.type === "victory") && view.lovers.length === 2 && (
        <p>{fr.game.lovers(name(view.lovers[0]!), name(view.lovers[1]!))}</p>
      )}
      {/* Only a Spectator or everyone on the victory screen knows who the Thief robbed */}
      {view.theft && (
        <p>{fr.game.theft(name(view.theft.thief), view.theft.robbed === null ? null : name(view.theft.robbed))}</p>
      )}
      {/* Only a Spectator knows whom the Assassin chose until the next Night falls */}
      {view.assassinTarget !== null && <p>{fr.game.assassinTarget(name(view.assassinTarget))}</p>}
    </>
  );
}
