// What the Narrator says at each change of view: the Table device's script.

import type { Camp } from "./generated/Camp";
import type { Moment } from "./generated/Moment";
import type { PlayerId } from "./generated/PlayerId";
import type { PlayerView } from "./generated/PlayerView";
import type { Settings } from "./generated/Settings";
import { fr } from "./strings";

/** A Narrator line without names, spoken from its MP3. */
export type FixedLine = keyof typeof fr.narrator.fixed;

/** What the Table device does, in order. */
export type Cue =
  | { type: "fixed"; key: FixedLine; text: string }
  /** A line with names in it, spoken live. */
  | { type: "live"; text: string }
  /** A one-off sound. It follows a Moment, never a Player's choice. */
  | { type: "sound"; sound: Sound }
  /** The looped background changes to the Phase's, or fades out (`null`). */
  | { type: "background"; phase: Phase | null };

export type Sound = "howl" | "bottle" | "sword" | "gunshot" | "rooster";
export type Phase = "night" | "day";

/** What the Narrator's script reads from a view. */
export type NarratorView = Pick<PlayerView, "moment" | "players" | "paused"> & {
  settings: Pick<Settings, "hiddenRoles">;
};

type MomentType = Moment["type"];

const fixed = (key: FixedLine): Cue => ({ type: "fixed", key, text: fr.narrator.fixed[key] });
const live = (text: string): Cue => ({ type: "live", text });
const sound = (sound: Sound): Cue => ({ type: "sound", sound });

/** The line each Night Turn's Role goes back to sleep with. */
const SLEEP_LINES: Partial<Record<MomentType, FixedLine>> = {
  cupidsTurn: "cupidSleeps",
  loversTurn: "loversSleep",
  thiefsTurn: "thiefSleeps",
  seersTurn: "seerSleeps",
  werewolvesTurn: "werewolvesSleep",
  witchsTurn: "witchSleeps",
};

const VICTORY_LINES: Record<Camp, FixedLine> = {
  village: "villageWins",
  werewolves: "werewolvesWin",
  lovers: "loversWin",
  assassin: "assassinWins",
};

/** The Phase whose background plays at each Moment. None during the Role reveal. */
function phaseOf(type: MomentType): Phase | null {
  if (type === "roleReveal") return null;
  return type in SLEEP_LINES ? "night" : "day";
}

/**
 * The cues the Table device plays when its view goes from `previous` to
 * `next`. `previous` is `null` when there is nothing to follow on, like
 * the Table device taking its seat back. Silent while the Game is Paused,
 * the background faded out; on resume, the current Moment's line is said
 * again over its background, without its one-off sound: that marks the
 * Moment's start. Back in the Lobby, the background fades out.
 */
export function narratorCues(previous: NarratorView | null, next: NarratorView): Cue[] {
  const moment = next.moment;
  if (!moment) return previous?.moment ? [{ type: "background", phase: null }] : [];
  if (next.paused) return previous?.moment && !previous.paused ? [{ type: "background", phase: null }] : [];
  const before = (!previous?.paused && previous?.moment?.type) || null;
  // Changes within the same Moment, like the Seer's reading Moment, say nothing new.
  if (before === moment.type) return [];
  const cues: Cue[] = [];
  const sleep = before && SLEEP_LINES[before];
  if (sleep) cues.push(fixed(sleep));
  const phase = phaseOf(moment.type);
  if (phase && phase !== (before && phaseOf(before))) {
    cues.push({ type: "background", phase });
    if (phase === "night" && before) cues.push(fixed("nightfall"));
  }
  const announced = momentCues(moment, next);
  return [...cues, ...(before ? announced : announced.filter((cue) => cue.type !== "sound"))];
}

/** The current Moment's line, as every phone shows it. `null` in the Lobby. */
export function narratorText(view: NarratorView): string | null {
  if (!view.moment) return null;
  return momentCues(view.moment, view)
    .flatMap((cue) => ("text" in cue ? [cue.text] : []))
    .join(" ");
}

/** What the Narrator announces at `moment` itself. */
function momentCues(moment: Moment, { players, settings }: NarratorView): Cue[] {
  const name = (id: PlayerId) => players.find((p) => p.id === id)?.name ?? "?";
  // `null` while Hidden Roles keeps it secret, even from a Table device that
  // sees it: a dead Host is a Spectator, who sees every Role.
  const roleOf = (id: PlayerId) => {
    const role = players.find((p) => p.id === id)?.revealedRole;
    return role && !settings.hiddenRoles ? fr.roles[role].name : null;
  };
  const n = fr.narrator;
  switch (moment.type) {
    // The same line for the whole reveal.
    case "roleReveal":
      return [fixed("roleReveal")];
    case "cupidsTurn":
      return [fixed("cupidsTurn")];
    case "loversTurn":
      return [fixed("loversTurn")];
    case "thiefsTurn":
      return [fixed("thiefsTurn")];
    case "seersTurn":
      return [fixed("seersTurn")];
    case "werewolvesTurn":
      return [sound("howl"), fixed("werewolvesTurn")];
    case "witchsTurn":
      return [sound("bottle"), fixed("witchsTurn")];
    case "dawn":
      return [
        sound("rooster"),
        fixed("dawn"),
        moment.deaths.length === 0
          ? fixed("dawnNobody")
          : live(n.dawnDeaths(moment.deaths.map((id) => ({ name: name(id), role: roleOf(id) })))),
        ...(moment.robbery ? [fixed("robbery")] : []),
      ];
    case "discussion":
      return [fixed("discussion")];
    case "election":
      return [fixed("election")];
    case "electionResult":
      return [live(moment.byLot ? n.electedByLot(name(moment.mayor)) : n.elected(name(moment.mayor)))];
    case "tieBreak":
      return [live(n.tied(moment.tied.map(name).join(", "))), fixed("tieBreak")];
    case "vote":
      return [fixed("vote")];
    case "voteResult":
      return moment.eliminated === null
        ? [fixed("voteNobody")]
        : [sound("sword"), live(n.voteEliminated(name(moment.eliminated), roleOf(moment.eliminated)))];
    case "huntersShot":
      return [live(n.hunter(name(moment.hunter))), fixed("huntersShot")];
    case "shotResult":
      return moment.shot === null
        ? [fixed("shotLost")]
        : [sound("gunshot"), live(n.shot(name(moment.shot), roleOf(moment.shot)))];
    case "succession":
      return [live(n.succession(name(moment.mayor)))];
    case "successionResult":
      return [
        live(moment.byLot ? n.successorByLot(name(moment.successor)) : n.successor(name(moment.successor))),
      ];
    case "grief":
      return [live(n.grief(name(moment.lover), roleOf(moment.lover)))];
    case "victory":
      // No winner: nobody is left alive.
      return [fixed(moment.winner ? VICTORY_LINES[moment.winner] : "nobodySurvived")];
  }
}
