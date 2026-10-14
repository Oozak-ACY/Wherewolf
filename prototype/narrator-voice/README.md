# PROTOTYPE: Narrator voice via ElevenLabs (throwaway)

**Question:** ADR 0003 moves the Narrator's voice from browser speech to a cloud voice, with ElevenLabs as the candidate. On the Table device (the Host's phone on loudspeaker), does ElevenLabs sound good enough, start fast enough, and cost little enough to replace browser speech?

## Run it

1. Copy `.env.example` to `.env` and paste an ElevenLabs API key into it. The key stays on your computer: the phone only talks to the local proxy.
2. Install and start the server:
   ```bash
   cd prototype/narrator-voice && npm install && npm run proto
   ```
3. On the phone (same Wi-Fi), open the **Network** URL that Vite prints. Plain http is fine because no mic is needed.
4. Tap **Unlock audio** once, then put the phone on the table at full volume.

## What to try

- **Voice:** pick a French voice from the Voice Library group, preview it, then **add** it (the API can only speak with voices in "My voices"). Compare it with *Browser speech*.
- **Model:** flash vs multilingual v2 vs v3. Each one trades speed for expressiveness.
- **Scripted Night + Day:** plays 13 lines with no taps, pausing while Players "act on their phones". This checks that iOS keeps playing without a tap, and how long the table waits after each trigger. Run it with prefetch on and off.
- **Warm cache:** pre-generates the fixed lines. After that, only lines with names (deaths, Mayor, eliminations) need a live request.

## Bring back

For each phone you test (model and browser): the median latency for a cache miss and a cache hit for each model, whether the scripted run ever got stuck without a tap, which voice and model sounded right, and the credits used for one scripted Night + Day.

## Ambiance

Sounds live in `public/audio` (see `CREDITS.md`) and load when you tap **Unlock audio**. The scripted run switches the background (Night: music + wind, Day: tavern) and plays the howl, the slash and the gunshot at their Moments. The **Ambiance** panel has a sound board and a mixer: background and effect volumes, how far the background drops under the Narrator, and the Night/Day crossfade time. iOS ignores volume set on an `<audio>` tag, so the Narrator always plays at full volume and the mix is balanced against it.

## Verdict (2026-10-14)

**Yes: an ElevenLabs Narrator plus a sound mix on the Table device is the way to go.** Tested by the Host at the table on their phone.

- **Voice:** "Narrateur sombre", a voice made with ElevenLabs Voice Design (free plan; the prompt is in the session notes below). Voice Library voices are refused by the API on the free plan.
- **Model:** fixed lines are pre-generated once as MP3s with `eleven_v3` (most expressive) and shipped with the client: instant and free per Game. Lines with names are generated live with `eleven_flash_v2_5`.
- **Latency measured:** a live line took 1.5 s (flash), 2.1 s (v3) and 2.8 s (multilingual v2) before ElevenLabs sent its first audio. A cached line starts in about 60 ms. Live lines are generated while the fixed line before them plays.
- **Cost:** about 560 characters of speech for a whole Night + Day, almost all of it fixed lines, so it's paid once.
- **Ambiance:** Night = suspense music + wind (looped, layered), Day = tavern ambience, crossfaded. One-off sounds: howl (Werewolves' Turn), bottle smash (Witch's Turn), sword slash (Vote elimination), gunshot (Hunter's shot), kiss (Cupid's Turn, future Role). The background dips under the Narrator.
- **Rule:** a sound follows a Moment, never a Player's choice: the bottle plays when the Witch wakes, never when she uses a potion.
- **Host's mixing table:** three faders, 0–150%: Narrator, Music & ambiance, Sound effects. The voice must go through Web Audio, because iOS ignores `audio.volume`. A limiter at the end keeps boosted levels from clipping.

Voice Design prompt used:

> Native French, français de France with a neutral Parisian accent. Male, in his 60s. Studio-quality recording. Persona: village storyteller, night narrator. Emotion: calm, mysterious, gravely solemn. A deep, warm and slightly gravelly voice with a velvety low register, close to the microphone. He speaks slowly and deliberately, with clear articulation and natural pauses between sentences, letting each announcement land. His tone stays hushed and suspenseful at night, then grows firmer and more authoritative when he announces a death or a verdict, without ever shouting.
