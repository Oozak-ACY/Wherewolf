# The Narrator speaks with pre-generated ElevenLabs lines, over a sound mix on the Table device

At the table, the Narrator's voice and the room's mood both come from the Table device. Browser speech sounded robotic and varied from phone to phone, so the Narrator now speaks with one ElevenLabs voice ("Narrateur sombre", made with Voice Design). Every line without a name is generated once, as an MP3 with `eleven_v3`, and shipped with the client: it starts instantly and costs nothing per Game. A line with a name in it (deaths, Mayor, eliminations, the Hunter's shot) is generated live with `eleven_flash_v2_5` through the game server, which holds the API key. It is requested as soon as the outcome is known, so it is mostly ready by the time the fixed line before it ends.

Under the voice plays a sound mix: a looped background for the Phase (Night: suspense music and wind, Day: tavern), crossfaded at nightfall and dawn, and one-off sounds at given Moments (a howl when the Werewolves' Turn starts, a bottle smash when the Witch's starts, a rooster crow at dawn, a sword slash for an elimination by Vote, a gunshot for the Hunter's shot). The background dips while the Narrator speaks. The Host sets three levels, Narrator, Music & ambiance and Sound effects, on their phone. Every other phone stays silent.

**A sound follows a Moment, never a Player's choice:** the bottle smash plays whenever the Witch's Turn starts, never when she uses a potion, since otherwise the table would hear what she did.

Validated at the table by the `prototype/narrator-voice` branch (verdict in its `prototype/narrator-voice/README.md`).

## Considered options

- **Everything live with ElevenLabs:** rejected. A live line takes 1.5 to 3 s before sound starts, and every Game would pay for the same fixed lines again.
- **One model for everything:** rejected. `eleven_v3` sounds best but is slower live, and `eleven_flash_v2_5` is the fastest. Live lines are short, so a small difference in delivery is acceptable.
- **Voice Library voices:** not used. The API refuses them on the free plan.
- **Playing the voice through a plain `<audio>` element:** rejected. iOS ignores its volume, so the Host could not set the Narrator's level. The voice goes through Web Audio with the rest of the mix, ending in a limiter.

## Consequences

- The game server needs `ELEVENLABS_API_KEY` (Render) and a live text-to-speech endpoint. The client never sees the key.
- Changing a fixed Narrator line means regenerating its MP3. A script regenerates them all from `strings.ts`.
- The Narrator needs "goes back to sleep" lines for each Role, as players close their eyes for real.
- Sound files come from Pixabay; their origin is listed next to them in a `CREDITS.md`.
- If ElevenLabs fails for a live line, the Narrator must still say something: browser speech is the fallback.
