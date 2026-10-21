import { useState, type ReactNode } from "react";
import { Choice, Footer, PickRows } from "./Choice";
import { GameNotes } from "./GamePanel";
import type { Moment } from "./generated/Moment";
import type { Pick } from "./generated/Pick";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerSummary } from "./generated/PlayerSummary";
import type { PlayerView } from "./generated/PlayerView";
import type { Potions } from "./generated/Potions";
import { picking, type Picking } from "./ground";
import type { NightTurn } from "./nightTurn";
import { cardImageUrl, partnerOf } from "./RoleCardView";
import { fr } from "./strings";

type Props = {
  view: PlayerView;
  moment: Moment;
  turn: NightTurn;
  banners: ReactNode;
  onChooseLovers: (lovers: PlayerId[]) => void;
  onConfirmPartner: () => void;
  onRob: (player: PlayerId) => void;
  onInspect: (player: PlayerId) => void;
  onPickVictim: (victim: PlayerId) => void;
  onHeal: () => void;
  onPass: () => void;
  onPoison: (player: PlayerId) => void;
  onLeave: () => void;
};

/**
 * A Night Turn as the acting Player or a Spectator sees it, under the
 * Narrator bar: the card and title of who acts, the prompt, the pick rows
 * and the footer. Every Turn shares this layout.
 */
export function NightTurnView(props: Props) {
  const { view, moment, turn } = props;
  const name = (id: PlayerId) => view.players.find((p) => p.id === id)?.name ?? "?";
  const me = view.players.find((p) => p.id === view.you);
  const living = view.players.filter((p) => p.alive);
  const others = living.filter((p) => p.id !== view.you);
  const partner = partnerOf(view);
  const mode = picking(view);

  const head = (prompt: ReactNode, hint?: ReactNode) => (
    <header className="turn-head">
      <img className="card-art thumb" src={cardImageUrl(turn.card)} alt="" />
      <div className="turn-text">
        <h1>{turn.title}</h1>
        {prompt}
        {hint}
      </div>
    </header>
  );
  const choosing = (line: string) => head(<p className="muted">{line}</p>);

  let content: ReactNode = head(null);
  switch (moment.type) {
    case "cupidsTurn":
      content = mode ? (
        <CupidsPick prompt={head(<p>{fr.game.cupidPick}</p>)} living={living} onChooseLovers={props.onChooseLovers} />
      ) : (
        choosing(fr.game.cupidChoosing)
      );
      break;

    case "loversTurn":
      if (partner !== undefined && me?.alive) {
        content = (
          <>
            {head(<p>{fr.game.partner(name(partner))}</p>)}
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
        );
      }
      break;

    case "thiefsTurn":
      content = mode ? (
        <>
          {head(<p>{fr.game.thiefPick}</p>)}
          <Choice mode={mode} players={others} confirmLabel={fr.game.confirm.rob} onChoose={props.onRob}>
            <button className="quiet" onClick={props.onPass}>
              {fr.game.thiefPass}
            </button>
          </Choice>
        </>
      ) : (
        choosing(fr.game.thiefChoosing)
      );
      break;

    case "seersTurn":
      content = moment.inspection ? (
        head(<p>{fr.game.seerSaw(name(moment.inspection.player), fr.roles[moment.inspection.role].name)}</p>)
      ) : moment.passed ? (
        choosing(fr.game.seerPassed)
      ) : mode ? (
        <>
          {head(<p>{fr.game.seerPick}</p>)}
          <Choice
            mode={mode}
            players={others}
            mark={<EyeIcon />}
            confirmLabel={fr.game.confirm.inspect}
            onChoose={props.onInspect}
          >
            <button className="quiet" onClick={props.onPass}>
              {fr.game.seerPass}
            </button>
          </Choice>
        </>
      ) : (
        choosing(fr.game.seerSawNothing)
      );
      break;

    case "werewolvesTurn":
      if (!moment.picks) break;
      content = mode ? (
        <>
          {head(<p>{fr.game.werewolvesPick}</p>, <p className="muted hint">{fr.game.unanimity}</p>)}
          <WerewolvesPick
            mode={mode}
            picks={moment.picks}
            you={view.you}
            living={living}
            name={name}
            onPickVictim={props.onPickVictim}
            onPass={props.onPass}
          />
        </>
      ) : (
        <>
          {head(null, <p className="muted hint">{fr.game.unanimity}</p>)}
          <ul className="ballots">
            {moment.picks.map((p) => (
              <li key={p.werewolf}>{fr.game.pick(name(p.werewolf), p.victim === null ? null : name(p.victim))}</li>
            ))}
          </ul>
        </>
      );
      break;

    case "witchsTurn": {
      const witch = moment.witch;
      if (!witch) break;
      const potions = me?.alive && !witch.passed ? view.role?.potions : null;
      content = (
        <>
          {head(
            <>
              <p>{witch.victim === null ? fr.game.witchNoVictim : fr.game.witchVictim(name(witch.victim))}</p>
              {witch.healed && witch.victim !== null && <p>{fr.game.witchHealed(name(witch.victim))}</p>}
              {witch.poisoned !== null && <p>{fr.game.witchPoisoned(name(witch.poisoned))}</p>}
              {witch.passed && <p className="muted">{fr.game.witchPassed}</p>}
            </>,
            potions && <p className="muted hint">{fr.game.potionsLeft(potions.healing, potions.poison)}</p>,
          )}
          {potions && (
            <WitchsChoice
              mode={mode}
              potions={potions}
              victim={witch.victim}
              living={living}
              name={name}
              onHeal={props.onHeal}
              onPoison={props.onPoison}
              onPass={props.onPass}
            />
          )}
        </>
      );
      break;
    }
  }

  return (
    <section className="night-turn">
      {props.banners}
      {content}
      {/* Only a Spectator has notes at Night and they have no footer */}
      <GameNotes view={view} moment={moment} />
      {view.spectating && (
        <button className="quiet" onClick={props.onLeave}>
          {fr.lobby.leave}
        </button>
      )}
    </section>
  );
}

/** Cupid's choice of the two Lovers, any two living Players, sent once both are chosen. */
function CupidsPick(props: {
  prompt: ReactNode;
  living: PlayerSummary[];
  onChooseLovers: (lovers: PlayerId[]) => void;
}) {
  const [chosen, setChosen] = useState<PlayerId[]>([]);
  // A third choice replaces the oldest one.
  const toggle = (id: PlayerId) =>
    setChosen((now) => (now.includes(id) ? now.filter((c) => c !== id) : [...now, id].slice(-2)));
  return (
    <>
      {props.prompt}
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
      {potions.healing && victim !== null && <button onClick={props.onHeal}>{fr.game.heal(props.name(victim))}</button>}
      <button className="quiet" onClick={props.onPass}>
        {fr.game.pass}
      </button>
    </>
  );
  return props.mode ? (
    <>
      <p className="choice-label">{fr.game.poisonPick}</p>
      <Choice mode={props.mode} players={props.living} confirmLabel={fr.game.confirm.poison} onChoose={props.onPoison}>
        {actions}
      </Choice>
    </>
  ) : (
    <Footer>{actions}</Footer>
  );
}

function EyeIcon() {
  return (
    <svg className="pick-mark" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" aria-hidden="true">
      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}
