// Every piece of text shown to Players, in one place so a second language can
// be added later.

import type { Camp } from "./generated/Camp";
import type { Rejection } from "./generated/Rejection";
import type { Role } from "./generated/Role";
import type { Timers } from "./generated/Timers";
import type { MicProblem } from "./useCall";

// Mirror MIN_PLAYERS, MAX_PLAYERS, MAX_WITCHES, MAX_CUPIDS, MAX_THIEVES, TIMER_BOUNDS
// and GRACE_PERIOD_BOUNDS (the same as TIMER_BOUNDS) in crates/engine/src/lib.rs.
export const MIN_PLAYERS = 5;
export const MAX_PLAYERS = 12;
export const MAX_WITCHES = 1;
export const MAX_CUPIDS = 1;
export const MAX_THIEVES = 1;
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
    showQrCode: "Afficher le QR code",
    qrCodeLabel: "QR code pour rejoindre le salon",
    qrCodeHint: "Touche l'écran pour revenir au salon",
    players: (count: number) => `Joueurs (${count} / ${MAX_PLAYERS})`,
    you: "toi",
    host: "Hôte",
    mayor: "Maire",
    offline: "déconnecté",
    leave: "Quitter le salon",
  },

  narratorBar: {
    label: (phase: "day" | "night", night: number) =>
      night > 0 ? `${phase === "night" ? "Nuit" : "Jour"} ${night} · Le Narrateur` : "Le Narrateur",
  },

  mixer: {
    open: "Volumes",
    title: "Volumes de la table",
    narrator: "Narrateur",
    ambiance: "Musique et ambiance",
    effects: "Effets sonores",
    test: "Son de test",
    close: "Fermer",
  },

  settings: {
    title: "Paramètres",
    roles: "Rôles",
    rolesCount: (roles: number, players: number) =>
      `${roles} rôle${roles > 1 ? "s" : ""} pour ${players} joueur${players > 1 ? "s" : ""}`,
    suggested: "Composition suggérée",
    timers: "Minuteries",
    gracePeriod: "Délai de reconnexion",
    gracePeriodHint: "Un joueur déconnecté plus longtemps perd sa place dans le salon.",
    hiddenRoles: "Rôles cachés",
    hiddenRolesOn: "Le rôle des morts reste caché aux vivants : le tour de nuit d'un rôle mort dure un temps tiré au hasard.",
    hiddenRolesOff: "Le rôle de chaque mort est révélé à tous.",
    seconds: "s",
    less: (role: string) => `Un ${role} de moins`,
    more: (role: string) => `Un ${role} de plus`,
    timer: {
      discussion: "Débat du jour",
      election: "Élection du Maire",
      vote: "Vote du village",
      cupid: "Tour de Cupidon",
      lovers: "Tour des Amoureux",
      thief: "Tour du Voleur",
      seer: "Tour de la Voyante",
      werewolves: "Tour des Loups-Garous",
      witch: "Tour de la Sorcière",
      hunter: "Tir du Chasseur",
      mayorSuccessor: "Succession du Maire",
      mayorTieBreak: "Égalité tranchée par le Maire",
      feignedTurnMinimum: "Tour feint, au moins (rôles cachés)",
    } satisfies Record<keyof Timers, string>,
    start: "Lancer",
    waitingForHost: "L'hôte lancera la partie.",
  },

  game: {
    myCard: "Ma carte",
    confirmRole: "J'ai vu mon rôle",
    stillReading: (reading: number) =>
      reading === 1 ? "1 joueur lit encore son rôle." : `${reading} joueurs lisent encore leur rôle.`,
    roleConfirmed: "La nuit tombera quand tout le monde aura vu son rôle.",
    endDebateToElection: "Passer à l'élection du maire",
    endDebateToVote: "Passer au vote",
    spectator: "spectateur",
    cupidPick: "Choisis les deux Amoureux. Tu peux te choisir.",
    cupidConfirm: "Unir ces deux Amoureux",
    cupidChoosing: "Cupidon choisit les Amoureux…",
    partner: (name: string) => `Tu es amoureux de ${name}.`,
    partnerConfirm: "J'ai vu mon Amoureux",
    partnerConfirmed: "Ton Amoureux le découvre à son tour.",
    lovers: (first: string, second: string) => `${first} et ${second} sont les Amoureux.`,
    thiefPick: "Choisis le joueur à voler : à l'aube, tu prendras son rôle et il deviendra Villageois.",
    thiefPass: "Ne voler personne",
    thiefChoosing: "Le Voleur choisit…",
    theft: (thief: string, robbed: string | null) =>
      robbed ? `${thief}, le Voleur, a volé le rôle de ${robbed}.` : `${thief} est le Voleur.`,
    seerPick: "Choisis le joueur dont tu veux découvrir le rôle.",
    seerSaw: (name: string, role: string) => `${name} est ${role}.`,
    seerSawNothing: "La Voyante n'a encore rien vu.",
    seerPass: "N'inspecter personne cette nuit",
    seerPassed: "La Voyante n'inspecte personne cette nuit.",
    werewolvesPick: "Choisis la victime avec les autres Loups-Garous.",
    unanimity:
      "Il faut que tous les Loups-Garous choisissent la même victime. S'ils ne dévorent personne, personne ne meurt.",
    pickedBy: (names: string[]) => `choisi par ${names.join(", ")}`,
    werewolvesPass: "Ne dévorer personne",
    pick: (werewolf: string, victim: string | null) => `${werewolf} → ${victim ?? "personne"}`,
    witchVictim: (name: string) => `Les Loups-Garous ont choisi ${name}.`,
    witchNoVictim: "Les Loups-Garous n'ont choisi personne cette nuit.",
    witchHealed: (name: string) => `${name} est sauvé par la potion de guérison.`,
    witchPoisoned: (name: string) => `${name} est empoisonné.`,
    heal: "Utiliser la potion de guérison",
    poisonPick: "Utiliser la potion de poison sur :",
    pass: "Ne plus rien faire cette nuit",
    witchPassed: "La Sorcière ne fait plus rien cette nuit.",
    potionsLeft: (healing: boolean, poison: boolean) =>
      healing && poison
        ? "Il te reste tes deux potions."
        : healing
          ? "Il te reste la potion de guérison."
          : poison
            ? "Il te reste la potion de poison."
            : "Tu as utilisé tes deux potions.",
    asleep: "Tu dors : garde les yeux fermés jusqu'à ce que le Narrateur t'appelle.",
    narratorSpeaking: "Le Narrateur parle…",
    electPrompt: "Qui doit être Maire ?",
    votePrompt: "Qui faut-il éliminer ?",
    votedCount: (voted: number, living: number) => `${voted} / ${living} ont voté`,
    abstain: "S'abstenir",
    yourVote: (name: string) => `Ton vote : ${name}`,
    youAbstained: "Tu t'abstiens.",
    ballot: (voter: string, designated: string | null) =>
      designated ? `${voter} → ${designated}` : `${voter} : abstention`,
    spectating:
      "Tu es spectateur : tu vois toute la partie, chaque rôle et chaque tour de nuit. Reste silencieux à la table, et tu ne peux plus agir ni voter.",
    shootPick: "Tu es mort : choisis le joueur que tu emportes avec toi.",
    shootPass: "Ne tirer sur personne",
    tieBreakPick: "Tu es le Maire : choisis qui est éliminé.",
    tieBreakPass: "N'éliminer personne",
    mayorChoosing: (mayor: string) => `${mayor}, le Maire, choisit…`,
    successorPick: "Tu es mort : désigne le nouveau Maire.",
    mayorNaming: (mayor: string) => `${mayor} désigne son successeur…`,
    hunterAiming: (hunter: string) => `${hunter} choisit sur qui tirer…`,
    everyRole: "Les rôles de chacun",
    playAgain: "Rejouer",
    waitingForHostToPlayAgain: "L'hôte peut relancer une partie.",
    paused: (host: string) =>
      `Partie en pause : le téléphone de ${host}, qui porte la voix du Narrateur, s'est déconnecté. Elle reprendra où elle s'est arrêtée.`,
  },

  // What the Narrator announces at each moment, shown in the banner and read aloud
  // on the Table device. Keep them short.
  narrator: {
    // Lines without names: each one is spoken from its MP3 (see
    // scripts/generate-narrator-lines.ts), so changing one means regenerating it.
    fixed: {
      roleReveal: "Chacun découvre son rôle en secret sur son téléphone.",
      nightfall: "La nuit tombe.",
      cupidsTurn: "Cupidon se réveille.",
      cupidSleeps: "Cupidon a décoché ses flèches et se rendort.",
      loversTurn: "Les Amoureux se réveillent.",
      loversSleep: "Les Amoureux se rendorment, le cœur battant.",
      thiefsTurn: "Le Voleur se réveille.",
      thiefSleeps: "Le Voleur se rendort sans un bruit.",
      seersTurn: "La Voyante se réveille.",
      // Each Role goes back to sleep with the same line whatever it did.
      seerSleeps: "La Voyante referme les yeux sur ce qu'elle a vu.",
      werewolvesTurn: "Les Loups-Garous se réveillent.",
      werewolvesSleep: "Les Loups-Garous regagnent l'ombre et se rendorment.",
      witchsTurn: "La Sorcière se réveille.",
      witchSleeps: "La Sorcière range ses fioles et se rendort.",
      dawn: "Le jour se lève.",
      dawnNobody: "Personne n'est mort cette nuit.",
      // The same line whether or not a Theft took place.
      robbery: "Il semble qu'une maison ait été cambriolée cette nuit, vérifiez votre carte.",
      discussion: "Le village débat : qui sont les Loups-Garous ?",
      election: "Le village élit son Maire.",
      tieBreak: "Le Maire choisit qui est éliminé.",
      vote: "Le village vote.",
      voteNobody: "Personne n'est éliminé.",
      huntersShot: "Il choisit sur qui tirer.",
      shotLost: "Le Chasseur n'a tiré sur personne.",
      villageWins: "Le Village gagne : les Loups-Garous sont morts.",
      werewolvesWin: "Les Loups-Garous gagnent : le village est à eux.",
      loversWin: "Les Amoureux gagnent : seuls au monde, ils vivront leur amour.",
      nobodySurvived: "Personne n'a survécu.",
    },
    // Lines with names, spoken live.
    // A role is `null` while Hidden Roles keeps it secret.
    dawnDeaths: (dead: { name: string; role: string | null }[]) => {
      const withRoles = dead.every(({ role }) => role);
      const named = dead.map(({ name, role }) => (withRoles ? `${name}, qui était ${role}` : name));
      // "qui était" clauses need "et" to show where the last one starts.
      const list =
        withRoles && named.length > 1
          ? `${named.slice(0, -1).join(", ")}, et ${named[named.length - 1]}`
          : named.join(", ");
      return `Cette nuit, le village a perdu ${list}.`;
    },
    elected: (name: string) => `${name} est élu Maire.`,
    electedByLot: (name: string) => `Le sort désigne ${name} comme Maire.`,
    tied: (tied: string) => `Égalité entre ${tied}.`,
    voteEliminated: (name: string, role: string | null) =>
      role ? `Le village a éliminé ${name}, qui était ${role}.` : `Le village a éliminé ${name}.`,
    hunter: (hunter: string) => `${hunter}, le Chasseur, emporte quelqu'un avec lui.`,
    shot: (name: string, role: string | null) =>
      role ? `Le Chasseur a tiré sur ${name}, qui était ${role}.` : `Le Chasseur a tiré sur ${name}.`,
    succession: (mayor: string) => `${mayor}, le Maire, désigne son successeur.`,
    successor: (name: string) => `${name} devient Maire.`,
    successorByLot: (name: string) => `Le sort désigne ${name} comme nouveau Maire.`,
    grief: (name: string, role: string | null) =>
      role ? `${name}, qui était ${role}, meurt de chagrin.` : `${name} meurt de chagrin.`,
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
    cupid: {
      name: "Cupidon",
      power: "La première nuit, tu désignes deux Amoureux, toi compris si tu veux.",
    },
    thief: {
      name: "Voleur",
      power:
        "La deuxième nuit, tu peux voler le rôle d'un joueur : à l'aube, tu prends sa carte telle quelle et il devient Villageois. Sinon, tu restes Villageois.",
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
    lovers: { name: "Amoureux", goal: "Être, ton Amoureux et toi, les deux derniers survivants." },
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
    holdToTalk: "Maintenir pour parler",
    muteMe: "Couper mon micro",
    unmuteMe: "Réactiver mon micro",
    openMic: "Micro ouvert",
  },

  rejection: {
    lobbyFull: `Le salon est complet : ${MAX_PLAYERS} joueurs maximum.`,
    notSeated: "Tu n'as pas de place dans ce salon.",
    invalidName: "Choisis un prénom de 1 à 20 caractères.",
    notHost: "Seul l'hôte peut faire ça.",
    invalidSettings: `Paramètres refusés : ${MAX_PLAYERS} rôles au plus, une seule Sorcière, un seul Cupidon, un seul Voleur, et chaque minuterie (comme le délai de reconnexion) entre ${TIMER_MIN_SECONDS} s et ${TIMER_MAX_SECONDS / 60} min.`,
    notEnoughPlayers: `Il faut au moins ${MIN_PLAYERS} joueurs.`,
    roleCountMismatch: "Il faut autant de rôles que de joueurs.",
    playersDisconnected: "Attends que tous les joueurs soient reconnectés.",
    gameStarted: "La partie a déjà commencé.",
    notNow: "Ce n'est pas le moment.",
    notYourTurn: "Ce n'est pas ton tour.",
    potionUsed: "Tu as déjà utilisé cette potion.",
    noVictim: "Personne n'est à sauver cette nuit.",
    spectating: "Tu es mort : tu ne peux plus agir.",
    notInPlay: "Ce joueur n'est plus en jeu.",
    yourself: "Tu ne peux pas te choisir toi-même.",
    notTwoPlayers: "Choisis deux joueurs différents.",
    notTied: "Choisis parmi les joueurs à égalité.",
    paused: "La partie est en pause jusqu'au retour de l'hôte.",
  } satisfies Record<Rejection, string>,
};
