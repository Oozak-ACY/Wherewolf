# Handoff: medieval stained-glass redesign → `client/`

Read this before you implement anything. It covers what was designed, what is in this folder, and the decisions behind it. Nothing in `client/` has been touched yet. The whole job is to bring this look into `client/`.

- **Source canvas:** https://claude.ai/artifact/B8nJXneWUwceiYVDRF3rmn. It is private to the owner, and this folder is a full export of it.
- **Branch:** `prototype/redesign`, cut from `main` at `b49104c`.
- **Domain vocabulary:** see `CONTEXT.md` at the repo root (Player, Host, Narrator, Table device, Night Turn, Vote, Mayor…). The UI copy is the French text in `client/src/strings.ts`, and the mockups reuse it word for word.

## The brief

- **Style:** the owner asked for an "old medieval" style, and supplied 12 role cards painted as stained glass.
- **Mobile first:** the app is used mostly on phones around one table. Every screen is designed at **390 px wide** with a **20 px gutter**.

## What's in this folder

| Path | What it is |
|---|---|
| `roles/*.webp` | The 12 role cards, at their original size (most are 1116×2000; `assassin` and `little-girl` are 768×1344). Each is **9:16** with its frame painted into the art. |
| `screens/*.dc.html` | The six screens as Design Component sources. `{{…}}` holes are filled by the small `renderVals()` class at the bottom of each file. Treat these files as the spec, not as code to copy (they need the canvas runtime, `support.js`, to render). Images point to `../roles/`. |
| `screens/canvas.json` | The canvas layout: frame sizes and the order of the screens. |
| `tokens/tokens.json` | Every color, font, size, radius and shadow, each with a note on where it is used. |
| `tokens/tokens.css` | The same tokens as `--ww-*` custom properties, plus the Google Fonts import. |
| `components/components.css` | The reusable pieces taken from the screens, as `.ww-*` classes. They are the reference for porting into `client/src/styles.css`. |
| `components/index.html` | A sheet showing every component on Day, Night and Reveal grounds. Open it over a local server, because relative links break from `file://` previews. It has **not been checked visually** in a browser yet. |

## Role cards

| File | Role (`Role` type) | French name | In the engine? |
|---|---|---|---|
| `werewolf.webp` | `werewolf` | Loup-Garou | yes |
| `seer.webp` | `seer` | Voyante | yes |
| `witch.webp` | `witch` | Sorcière | yes |
| `hunter.webp` | `hunter` | Chasseur | yes |
| `cupid.webp` | `cupid` | Cupidon | yes |
| `thief.webp` | `thief` | Voleur | yes |
| `villager.webp` | `villager` | Villageois | yes |
| `mayor.webp` | (a title, not a Role) | Maire | yes, as the Mayor title |
| `angel.webp` | — | Ange (guessed) | no |
| `little-girl.webp` | — | Petite Fille (guessed) | no |
| `pyromaniac.webp` | — | Pyromane (guessed) | no |
| `assassin.webp` | — | Assassin (guessed) | no |

The owner supplied the last four without names. The names are guesses and **not confirmed**. Don't add them to `Role`; only the gallery mockup shows them, under "À venir" (coming soon).

**What to change in the client:**

- **File names:** `client/src/RoleCardView.tsx` → `roleImageUrl()` loads `/roles/<role>.png`. These cards are `.webp`, so either change the extension there and copy the files into `client/public/roles/`, or convert them to PNG.
- **README:** `client/public/roles/README.md` still says "about 3:4" and lists `.png`. Update it to **9:16**.
- **Card ratio:** show a card in a 9:16 box with `object-fit: cover`. Never crop it to 3:4, which cuts off the painted frame.
- **Mayor card:** `mayor.webp` can go anywhere the Mayor title appears (the tie-break note in the Vote, the Mayor badge).

## Design tokens (summary; `tokens/tokens.json` has them all)

- **Three grounds, one per mood:**
  - **Parchment** `#EFE4CB`: daytime and public screens (Home, Lobby, Vote, gallery).
  - **Night** `#101A2C`: Night Turns.
  - **Reveal** `#1A1210`: near-black, for private screens (the Role card). It keeps a phone screen dim and hard to read over a shoulder.
- **Glass colors:**
  - **Oxblood** `#8A2A1F`: primary actions, the Day narrator bar, avatars, vote counts, titles.
  - **Cobalt** `#1F3F6E`: secondary actions, the lobby header, the main action at night.
  - **Gold** `#B88A2E` and `#D9B45A`: decor only (frames, dividers, button inner lines, avatar rings). **Never use gold for text on parchment**, because it fails contrast. On dark grounds, `#F0D58A` is the text gold.
- **Text:**
  - **Ink** `#2B1D12`: text, and the "lead line" borders.
  - **Muted** `#5A4632`: labels; it passes 4.5:1 on parchment.
- **Type:** three families, from Google Fonts (URL in the tokens).
  - **UnifrakturMaguntia** (blackletter): only the wordmark and screen titles, 36–64 px. Never body text, because it is hard to read small.
  - **Cinzel**: buttons, labels, room codes and section titles. Labels are uppercase with 0.18–0.32 em letter spacing.
  - **EB Garamond**: all reading text, 17–19 px. Narrator lines and captions are italic. It covers French accents well.
- **Shape:** 6 px radius on controls, 8–10 px on cards. Primary buttons are 56 px tall and secondary ones 48 px. Every target is at least 44 px. Picking rows are 56–60 px.

## Components (`components/components.css`)

- **`.ww-btn` (buttons):** a stained-glass pane, with a 2 px ink border (the lead) and an inset 2 px gold-light ring (the glass edge).
  - `--primary`: oxblood, 56 px tall.
  - `--secondary`: cobalt.
  - `--ghost`: transparent, for a "pass" or "abstain" choice.
  - `--icon`: 48×48, on the colored headers, with an `aria-label`.
  - On Night screens, primary becomes cobalt with a gold border.
- **`.ww-stepper`:** the 44×36 − / + buttons under each role in the Lobby.
- **`.ww-input`:** 52 px tall, vellum fill. The room code is typed in Cinzel with wide letter spacing.
- **`.ww-divider`:** gold rules on each side of "OU" (or a small four-point star).
- **`.ww-header`:** the cobalt lobby bar with a 3 px gold bottom rule. It holds the room code (Cinzel 34 px), and QR and share buttons.
- **`.ww-narrator` (the Narrator bar), on every in-Game screen:**
  - Contents: a Phase icon (sun or moon), a label like "JOUR 2 · LE NARRATEUR", the line being spoken in italics, and the timer in Cinzel gold-glow.
  - Colors: oxblood by day, `--night` (night-deep) by night.
  - It should show the same line `strings.ts → narrator` gives the banner today.
- **`.ww-footer`:** the sticky bottom bar for the screen's main action, with a gold top rule. Main actions always sit within thumb reach at the bottom.
- **`.ww-avatar`:** the Player's initial in Cinzel on an oxblood disc (cobalt at night), with a gold ring.
- **`.ww-player-tile`:** a Lobby tile in a two-column grid: name, plus an italic tag (Hôte · toi, prêt, déconnecté).
- **`.ww-pick` (a pick row):** one full-width `<button>` per Player, used for every choice (Night Turns, Election, Vote, Hunter's shot, Mayor's succession). It uses `aria-pressed` to show the selection:
  - By day: oxblood border plus a gold inset.
  - At night: gold-glow border.
  - A `.ww-count` badge can show votes, greyed (`--zero`) at 0.
- **`.ww-card`:** the role artwork, kept at 9:16.
  - `--reveal`: 236 px wide, with a gold ring and a deep shadow.
  - `--thumb`: 74 px, next to a Night Turn title.
  - `--off`: 45% opacity, for a role at count 0 in the Lobby.
- **`.ww-pill`:** the camp badge (Village / Loups-Garous / Amoureux).
- **`.ww-frame`:** the double frame of the Home screen (a gold border, with an ink outline 4 px outside it).

## The six screens (`screens/`)

1. **`Main.dc.html` (Accueil, home):**
   - Inside the double frame: "LES LOUPS-GAROUS" label, the "Wherewolf" wordmark, then three fanned cards (Voyante −11°, Loup-Garou in front, Villageois +11°) and the italic tagline.
   - At the bottom: "Créer un salon" (primary), the OU divider, and the join-by-code field with "Y aller" (secondary).
   - It has an `accent` tweak, so the primary color can be swapped between oxblood, cobalt, forest and violet.
2. **`Lobby.dc.html` (Salon, the Host's view):**
   - Cobalt header with the code, QR and share buttons.
   - "Joueurs (n / 12)" over a two-column player grid.
   - "Rôles", with a live "x rôles pour y joueurs" count in forest green when it matches, over a **sideways-scrolling row of role cards** (96 px wide, count badge at the top right, − / + under each, greyed at 0).
   - A row linking to "Minuteries et rôles cachés" (timers and hidden roles) on a settings sub-screen (not designed).
   - A sticky "Lancer la partie" (start the game).
   - Players who aren't the Host see the same screen without the steppers and Start button, and with `strings.settings.waitingForHost` in their place (not drawn).
3. **`RoleReveal.dc.html` (Révélation du rôle):**
   - Reveal ground: "TU ES", the card at 236 px, the name in blackletter gold, the camp pill, the power text.
   - The fellow Werewolves (or the Lover's partner) in a bordered box, and "J'ai vu mon rôle" at the bottom.
   - It has a `role` tweak that switches between the seven Roles.
   - It replaces `RoleCardView.tsx`'s overlay, so keep it a modal dialog, with focus on the button.
4. **`Night.dc.html` (Nuit, the Seer's turn):** the pattern for **every Night Turn**.
   - The night Narrator bar and timer, then the role thumbnail and title in blackletter, with the prompt from `strings.game.*Pick`.
   - A list of pick rows, then a footer with the confirm action (whose label names the choice) and a ghost "pass" action.
   - The Werewolves, Witch, Cupid and Thief reuse this layout. The Witch adds potion actions and Cupid picks two rows.
   - Players who are asleep keep `SleepingView` (not redesigned; give it the night ground).
5. **`Vote.dc.html` (Jour, the village vote):** the pattern for **Election, Vote and tie-break**.
   - The oxblood Day Narrator bar, then the title "Qui faut-il éliminer ?", then "Ton vote : …" next to "x / y ont voté".
   - Pick rows show who voted for that Player (italic) and a count badge.
   - A dashed note with the Mayor card explains the tie-break, and a sticky "S'abstenir" (abstain) ghost button sits at the bottom.
6. **`Roles.dc.html` (Galerie des rôles):** a two-column gallery, sorted into "En jeu" (in play: 7 Roles plus Maire) and "À venir" (coming soon: the 4 unnamed cards). It is a reference sheet. A rules or help screen in the app could reuse it, but nobody has asked for one.

## Design decisions and why

- **The cards carry the style.** The interface stays flat and quiet (parchment, ink lines, a few glass colors) so the art can be the richest thing on screen. There are no gradients, textures or glow effects.
- **Lead lines and glass:** every control has a dark outer border and a thin gold inner ring, to echo the lead between panes of glass. That one rule ties buttons, inputs, avatars and the reveal card together.
- **A ground for each mood:** parchment means public, daytime and safe to show; navy means Night; near-black means private. A Player can tell at a glance which kind of screen they hold.
- **One-handed use:** the main action is always in a sticky footer at the bottom, and choices are full-width rows rather than small tiles.
- **Blackletter only for titles:** it carries the period feel, but it is hard to read below ~30 px.
- **Contrast:** gold is decor only on light grounds; colors were checked to pass 4.5:1 for body text and labels.
- **Real controls:** use real `<button>`, `<input>` and `<label>` elements, icon buttons with `aria-label`, decorative SVGs with `aria-hidden`, and `aria-pressed` for selections.

## Not designed yet (decide while implementing, in the same language)

- The QR code view, `MixerControl` (volumes), and the timers and Hidden Roles settings sub-screen.
- `SleepingView`, the Spectator view, the Paused banner, connection-lost and error states (`strings.rejection`).
- The end of the Game ("Les rôles de chacun", the Camp that won, "Rejouer").
- `PlayerGrid` / `CallGrid` (remote-play tiles) and their `CardBack`: a stained-glass card back could be derived from the card frames.
- Desktop or tablet layouts: let the 390 px column sit centered, up to a max width of about 480 px.
- Light and dark mode: the design is meant to look the same under both. Set `<meta name="theme-color">` per ground: `#1F3F6E` or `#8A2A1F` by day, `#0A1220` by night.

## Suggested implementation order

1. Copy `roles/*.webp` into `client/public/roles/` and update `roleImageUrl()` and the README. That alone shows the art in today's UI.
2. Port `tokens.css` into the `:root` of `client/src/styles.css` (it currently holds the dark purple theme `--bg #14111f`…), and load the fonts in `client/index.html`.
3. Restyle the shared pieces (buttons, inputs, the Narrator banner, pick lists) to match `components.css`.
4. Screens in this order: Role reveal (`RoleCardView.tsx`), Home and Join (`App.tsx`), Lobby (`App.tsx` and `SettingsPanel.tsx`), Night Turns and Vote (`GamePanel.tsx`).
5. Keep `client/src/*.test.ts` passing. The tests check behavior (`RoleCardView.test.ts`, `sleepingScreen.test.ts`…), not looks.
