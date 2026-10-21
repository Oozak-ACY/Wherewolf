import { useState, type ReactNode } from "react";
import type { Potions } from "./generated/Potions";
import type { Ballot } from "./generated/Ballot";
import type { Moment } from "./generated/Moment";
import type { Pick } from "./generated/Pick";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import { picking, type Picking } from "./ground";
import { partnerOf } from "./RoleCardView";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
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
};

/**
 * The Game as this Player may see it, under the Narrator bar: what they can
 * do. At Night, only the acting Role and the Spectators get here; every other
 * phone shows the sleeping screen instead.
 */
export function GamePanel(props: Props) {
  const { view, moment } = props;
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const me = view.players.find((p) => p.id === view.you);
  const living = view.players.filter((p) => p.alive);
  const partner = partnerOf(view);
  const mode = picking(view);

  return (
    <section className="card stack game">
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

      {moment.type === "cupidsTurn" &&
        (mode ? (
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
              <Footer>
                <button className="primary" onClick={props.onConfirmPartner}>
                  {fr.game.partnerConfirm}
                </button>
              </Footer>
            )}
          </>
        ) : null)}

      {moment.type === "thiefsTurn" &&
        (mode ? (
          <>
            <p>{fr.game.thiefPick}</p>
            <Choice
              mode={mode}
              players={living.filter((p) => p.id !== view.you)}
              confirmLabel={fr.game.confirm.rob}
              onChoose={props.onRob}
            >
              <button className="quiet" onClick={props.onPass}>
                {fr.game.thiefPass}
              </button>
            </Choice>
          </>
        ) : (
          <p className="muted">{fr.game.thiefChoosing}</p>
        ))}

      {moment.type === "seersTurn" &&
        (moment.inspection ? (
          <p>{fr.game.seerSaw(name(moment.inspection.player), fr.roles[moment.inspection.role].name)}</p>
        ) : moment.passed ? (
          <p className="muted">{fr.game.seerPassed}</p>
        ) : mode ? (
          <>
            <p>{fr.game.seerPick}</p>
            <Choice
              mode={mode}
              players={living.filter((p) => p.id !== view.you)}
              confirmLabel={fr.game.confirm.inspect}
              onChoose={props.onInspect}
            >
              <button className="quiet" onClick={props.onPass}>
                {fr.game.seerPass}
              </button>
            </Choice>
          </>
        ) : (
          <p className="muted">{fr.game.seerSawNothing}</p>
        ))}

      {moment.type === "werewolvesTurn" &&
        (mode && moment.picks ? (
          <WerewolvesPick
            mode={mode}
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
              <WitchsChoice
                mode={mode}
                potions={view.role.potions}
                victim={moment.witch.victim}
                living={living}
                name={name}
                onHeal={props.onHeal}
                onPoison={props.onPoison}
                onPass={props.onPass}
              />
            )}
          </>
        ) : null)}

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

/** Cupid's choice of the two Lovers, any two living Players, sent once both are chosen. */
function CupidsPick(props: { living: PlayerSummary[]; onChooseLovers: (lovers: PlayerId[]) => void }) {
  const [chosen, setChosen] = useState<PlayerId[]>([]);
  // A third choice replaces the oldest one.
  const toggle = (id: PlayerId) =>
    setChosen((now) => (now.includes(id) ? now.filter((c) => c !== id) : [...now, id].slice(-2)));
  return (
    <>
      <p>{fr.game.cupidPick}</p>
      <PickRows players={props.living} pressed={(id) => chosen.includes(id)} onTap={toggle} />
      <Footer>
        <button className="primary" disabled={chosen.length !== 2} onClick={() => props.onChooseLovers(chosen)}>
          {fr.game.cupidConfirm}
        </button>
      </Footer>
    </>
  );
}

/** A living Werewolf's choice: a Victim, or nobody. Every Werewolf sees the others' picks. */
function WerewolvesPick(props: {
  mode: Picking;
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
      <Choice
        mode={props.mode}
        players={props.living}
        chosen={mine?.victim ?? null}
        detail={(id) => {
          const by = pickedBy(id);
          return by.length > 0 ? fr.game.pickedBy(by) : null;
        }}
        count={(id) => pickedBy(id).length}
        onChoose={props.onPickVictim}
      >
        <button
          className={mine && mine.victim === null ? "quiet chosen" : "quiet"}
          aria-pressed={mine?.victim === null}
          onClick={props.onPass}
        >
          <span>{fr.game.werewolvesPass}</span>
          {passedBy.length > 0 && <span className="muted"> {fr.game.pickedBy(passedBy)}</span>}
        </button>
      </Choice>
    </>
  );
}

/** The Witch's potions: the save in one tap, the poison picked then confirmed, or nothing more. */
function WitchsChoice(props: {
  mode: Picking | null;
  potions: Potions;
  victim: PlayerId | null;
  living: PlayerSummary[];
  name: (id: PlayerId) => string;
  onHeal: () => void;
  onPoison: (player: PlayerId) => void;
  onPass: () => void;
}) {
  const { potions, victim } = props;
  const actions = (
    <>
      {potions.healing && victim !== null && (
        <button onClick={props.onHeal}>{fr.game.heal(props.name(victim))}</button>
      )}
      <button className="quiet" onClick={props.onPass}>
        {fr.game.pass}
      </button>
    </>
  );
  return (
    <>
      <p className="muted">{fr.game.potionsLeft(potions.healing, potions.poison)}</p>
      {props.mode ? (
        <>
          <p>{fr.game.poisonPick}</p>
          <Choice
            mode={props.mode}
            players={props.living}
            confirmLabel={fr.game.confirm.poison}
            onChoose={props.onPoison}
          >
            {actions}
          </Choice>
        </>
      ) : (
        <Footer>{actions}</Footer>
      )}
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

/**
 * A choice of one Player among `players`, then the screen's footer with
 * `children` as its other actions. In `tap` mode each tap sends the choice,
 * `chosen` being the one sent. In `confirm` mode a tap only selects, and the
 * footer's confirm button, named by `confirmLabel`, sends it.
 */
function Choice(props: {
  mode: Picking;
  players: PlayerSummary[];
  chosen?: PlayerId | null;
  detail?: (id: PlayerId) => string | null;
  count?: (id: PlayerId) => number;
  confirmLabel?: (name: string) => string;
  onChoose: (id: PlayerId) => void;
  children?: ReactNode;
}) {
  const [selected, setSelected] = useState<PlayerId | null>(null);
  const confirming = props.mode === "confirm";
  // A selected Player who died meanwhile is no longer a choice.
  const picked = props.players.find((p) => p.id === selected);
  const pressed = confirming ? picked?.id : props.chosen;
  return (
    <>
      <PickRows
        players={props.players}
        pressed={(id) => id === pressed}
        detail={props.detail}
        count={props.count}
        onTap={confirming ? setSelected : props.onChoose}
      />
      {((confirming && picked) || props.children) && (
        <Footer>
          {confirming && picked && (
            <button className="primary" onClick={() => props.onChoose(picked.id)}>
              {props.confirmLabel?.(picked.name) ?? picked.name}
            </button>
          )}
          {props.children}
        </Footer>
      )}
    </>
  );
}

/** One full-width pick row per Player, with who picked them and how many. */
function PickRows(props: {
  players: PlayerSummary[];
  pressed: (id: PlayerId) => boolean;
  detail?: (id: PlayerId) => string | null;
  count?: (id: PlayerId) => number;
  onTap: (id: PlayerId) => void;
}) {
  return (
    <ul className="choices">
      {props.players.map((p) => {
        const detail = props.detail?.(p.id);
        const count = props.count?.(p.id);
        return (
          <li key={p.id}>
            <button className="pick-row" aria-pressed={props.pressed(p.id)} onClick={() => props.onTap(p.id)}>
              <span className="pick-name">
                <span>{p.name}</span>
                {detail && <span className="muted">{detail}</span>}
              </span>
              {count !== undefined && (
                <span className="pick-count" data-zero={count === 0}>
                  {count}
                </span>
              )}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** The screen's main actions, stuck to the bottom within thumb reach. */
function Footer(props: { children: ReactNode }) {
  return <div className="footer">{props.children}</div>;
}
