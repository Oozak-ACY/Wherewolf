// Every piece of text shown to Players, in one place so a second language can
// be added later.

import type { Rejection } from "./generated/Rejection";

// Mirrors MAX_PLAYERS in crates/engine/src/lib.rs.
export const MAX_PLAYERS = 12;

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

  rejection: {
    lobbyFull: `Le salon est complet : ${MAX_PLAYERS} joueurs maximum.`,
    notSeated: "Tu n'as pas de place dans ce salon.",
    invalidName: "Choisis un prénom de 1 à 20 caractères.",
  } satisfies Record<Rejection, string>,
};
