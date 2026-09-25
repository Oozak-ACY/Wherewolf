// Every piece of text shown to Players, in one place so a second language can
// be added later.

import type { Camp } from "./generated/Camp";
import type { Rejection } from "./generated/Rejection";
import type { Role } from "./generated/Role";
import type { Timers } from "./generated/Timers";
import type { MicProblem } from "./useCall";

// Mirror MIN_PLAYERS, MAX_PLAYERS and TIMER_BOUNDS in crates/engine/src/lib.rs.
export const MIN_PLAYERS = 5;
export const MAX_PLAYERS = 12;
export const TIMER_MIN_SECONDS = 10;
export const TIMER_MAX_SECONDS = 1800;

export const fr = {
  appName: "Wherewolf",
  tagline: "Les Loups-Garous, entre amis, à distance.",

  home: {
    create: "Créer un salon",
    creating: "Création…",
    createFailed: "Impossible de créer le salon. Le serveur se réveille peut-être : réessaie dans une minute.",
    joinWithCode: "Rejoindre avec un code",
    codePlaceholder: "Code du salon",
    go: "Y aller",
  },

  join: {
    title: (code: string) => `Salon ${code}`,
    nameLabel: "Ton prénom",
    namePlaceholder: "Ex. : Camille",
    submit: "Rejoindre",
    connecting: "Connexion…",
    lobbyNotFound: "Ce salon n'existe pas (ou plus). Vérifie le code.",
    connectionLost: "Connexion perdue. Nouvelle tentative…",
    backHome: "Retour à l'accueil",
  },

  lobby: {
    code: "Code",
    shareHint: "Envoie ce lien à tes amis :",
    copy: "Copier le lien",
    copied: "Lien copié !",
    share: "Partager",
    shareTitle: "Rejoins ma partie de Loups-Garous",
    players: (count: number) => `Joueurs (${count} / ${MAX_PLAYERS})`,
    you: "toi",
    host: "Hôte",
    offline: "déconnecté",
    leave: "Quitter le salon",
  },

  settings: {
    title: "Paramètres",
    roles: "Rôles",
    rolesCount: (roles: number, players: number) =>
      `${roles} rôle${roles > 1 ? "s" : ""} pour ${players} joueur${players > 1 ? "s" : ""}`,
    suggested: "Composition suggérée",
    timers: "Minuteries",
    seconds: "s",
    less: (role: string) => `Un ${role} de moins`,
    more: (role: string) => `Un ${role} de plus`,
    timer: {
      discussion: "Débat du jour",
      election: "Élection du Maire",
      vote: "Vote du village",
      seer: "Tour de la Voyante",
      werewolves: "Tour des Loups-Garous",
      witch: "Tour de la Sorcière",
      hunter: "Tir du Chasseur",
      mayorSuccessor: "Succession du Maire",
      mayorTieBreak: "Égalité tranchée par le Maire",
    } satisfies Record<keyof Timers, string>,
    start: "Lancer",
    waitingForHost: "L'hôte lancera la partie.",
  },

  game: {
    myCard: "Voir ma carte",
    dead: "mort",
    werewolvesPick: "Choisis la victime avec les autres Loups-Garous.",
    unanimity: "Il faut que tous les Loups-Garous choisissent la même victime.",
    pickedBy: (names: string[]) => `choisi par ${names.join(", ")}`,
    asleep: "Tu dors. Attends le lever du jour…",
    votePrompt: "Qui faut-il éliminer ?",
    votedCount: (voted: number, living: number) => `${voted} / ${living} ont voté`,
    abstain: "S'abstenir",
    yourVote: (name: string) => `Ton vote : ${name}`,
    youAbstained: "Tu t'abstiens.",
    ballot: (voter: string, designated: string | null) =>
      designated ? `${voter} → ${designated}` : `${voter} : abstention`,
    spectating: "Tu es mort. Tu ne peux plus agir ni voter.",
    everyRole: "Les rôles de chacun",
    playAgain: "Rejouer",
    waitingForHostToPlayAgain: "L'hôte peut relancer une partie.",
  },

  // What the Narrator announces at each moment, shown in the banner.
  narrator: {
    werewolvesTurn: "La nuit tombe. Les Loups-Garous se réveillent et choisissent leur victime…",
    dawnNobody: "Le jour se lève. Personne n'est mort cette nuit.",
    dawnDeaths: (victims: string) => `Le jour se lève. Cette nuit, le village a perdu ${victims}.`,
    discussion: "Le village débat : qui sont les Loups-Garous ?",
    vote: "Le village vote.",
    voteEliminated: (name: string, role: string) =>
      `Le village a éliminé ${name}, qui était ${role}.`,
    voteNobody: "Personne n'est éliminé.",
    victory: {
      village: "Victoire du Village ! Tous les Loups-Garous sont morts.",
      werewolves: "Victoire des Loups-Garous ! Ils ont pris le contrôle du village.",
    } satisfies Record<Camp, string>,
  },

  roleCard: {
    you: "Tu es",
    camp: "Camp",
    goal: "Objectif",
    fellowWerewolves: "Les autres Loups-Garous",
    loneWerewolf: "Tu es le seul Loup-Garou.",
    close: "Compris",
  },

  roles: {
    werewolf: {
      name: "Loup-Garou",
      power: "Chaque nuit, avec les autres Loups-Garous, tu choisis une victime à dévorer.",
    },
    seer: {
      name: "Voyante",
      power: "Chaque nuit, tu découvres le rôle d'un joueur de ton choix.",
    },
    witch: {
      name: "Sorcière",
      power:
        "Tu as une potion de guérison pour sauver la victime des Loups, et une potion de poison pour tuer. Chacune ne sert qu'une fois.",
    },
    hunter: {
      name: "Chasseur",
      power: "Quand tu meurs, tu abats aussitôt un joueur de ton choix.",
    },
    villager: {
      name: "Villageois",
      power: "Aucun pouvoir : seulement ton flair et ta voix pour démasquer les Loups.",
    },
  } satisfies Record<Role, { name: string; power: string }>,

  camps: {
    village: { name: "Village", goal: "Éliminer tous les Loups-Garous." },
    werewolves: {
      name: "Loups-Garous",
      goal: "Être au moins aussi nombreux que les autres joueurs encore en vie.",
    },
  } satisfies Record<Camp, { name: string; goal: string }>,

  call: {
    failed: "Impossible de rejoindre l'appel vidéo.",
    retry: "Réessayer",
    micRequired: "Le micro est obligatoire pour jouer.",
    micHelp: {
      refused:
        "Autorise l'accès au micro pour ce site dans ton navigateur, puis réessaie (recharge la page si rien ne se passe).",
      missing: "Aucun micro n'a été trouvé : branche-en un, puis réessaie.",
      unavailable: "Impossible d'utiliser le micro, peut-être déjà pris par une autre application : réessaie.",
    } satisfies Record<MicProblem, string>,
    enableSound: "Activer le son de l'appel",
  },

  rejection: {
    lobbyFull: `Le salon est complet : ${MAX_PLAYERS} joueurs maximum.`,
    notSeated: "Tu n'as pas de place dans ce salon.",
    invalidName: "Choisis un prénom de 1 à 20 caractères.",
    notHost: "Seul l'hôte peut faire ça.",
    invalidSettings: `Paramètres refusés : ${MAX_PLAYERS} rôles au plus, et chaque minuterie entre ${TIMER_MIN_SECONDS} s et ${TIMER_MAX_SECONDS / 60} min.`,
    notEnoughPlayers: `Il faut au moins ${MIN_PLAYERS} joueurs.`,
    roleCountMismatch: "Il faut autant de rôles que de joueurs.",
    gameStarted: "La partie a déjà commencé.",
    notNow: "Ce n'est pas le moment.",
    notYourTurn: "Ce n'est pas ton tour.",
    spectating: "Tu es mort : tu ne peux plus agir.",
    notInPlay: "Ce joueur n'est plus en jeu.",
  } satisfies Record<Rejection, string>,
};
