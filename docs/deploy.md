# Deploying Wherewolf

Three pieces, each on its own host:

| Piece | Host | Config |
| --- | --- | --- |
| Client (Vite + React) | Vercel, project root `client/` | `client/vercel.json`, env var `VITE_SERVER_URL` |
| Game server (Rust) | Render, free web service `wherewolf-server` | `render.yaml`, env vars `LIVEKIT_*`, `ELEVENLABS_API_KEY` |
| Video call | LiveKit Cloud | Keys live only on Render (and in your git-ignored `.env`) |
| Narrator's live lines | ElevenLabs | Key lives only on Render (and in your git-ignored `.env`) |

The client only knows the server's URL. The server is the only piece holding the LiveKit keys and hands each Player a call ticket (ADR 0001). It is also the only piece holding the ElevenLabs key: the Table device asks it to voice the Narrator's lines with names (`POST /api/lobbies/:code/live-line`), which it does only for the Host of a Game in progress. Without the key the server still runs, and the Table device says those lines with the browser's voice. The voice is "Narrateur sombre"; set `ELEVENLABS_VOICE_ID` on Render to use another.

## First deploy

Run the wizard. It walks you through Render and Vercel, then checks both work:

```bash
bash scripts/deploy-wizard.sh
```

It saves the two public URLs to a git-ignored `.env.deploy`, so a re-run offers them as defaults.

## Redeploying

Both hosts deploy automatically on every push to `main`:

- **Render** rebuilds the server only when `crates/`, `Cargo.toml`, `Cargo.lock` or `render.yaml` change (`buildFilter` in `render.yaml`). To redeploy by hand: Render → wherewolf-server → Manual Deploy → Deploy latest commit.
- **Vercel** rebuilds the client on every push. To redeploy by hand: Vercel → project → Deployments → ⋯ on the latest → Redeploy.

`VITE_SERVER_URL` is baked into the client at build time: after changing it in Vercel → Settings → Environment Variables, redeploy the client.

The free Render service sleeps after ~15 minutes without traffic. The first visit after that waits ~1 minute while it wakes up. Lobbies live in the server's memory, so a redeploy or a sleep ends any Lobby in progress.

## Adding the ElevenLabs key to an existing deploy

A Blueprint applied before `ELEVENLABS_API_KEY` was added to `render.yaml` doesn't have it yet. Render → wherewolf-server → Environment → Add Environment Variable: `ELEVENLABS_API_KEY`, with a key from ElevenLabs → Developers → API keys. Save: Render restarts the server. Its log no longer says `ELEVENLABS_API_KEY not set`.

## Rotating the ElevenLabs key

1. ElevenLabs → Developers → API keys → create a new key.
2. Render → wherewolf-server → Environment: replace `ELEVENLABS_API_KEY`, then Save.
3. Replace the same line in your local `.env`.
4. Play a Game until a dawn with a death: the Narrator says it in his own voice, not the browser's. Then delete the old key in ElevenLabs.

## Rotating the LiveKit keys

Do this if a secret leaks, or on a schedule. The same key is used locally and in production, so update both.

1. LiveKit Cloud → your project → Settings → API keys → create a new key. Copy its key and secret.
2. Render → wherewolf-server → Environment: replace `LIVEKIT_API_KEY` and `LIVEKIT_API_SECRET`, then Save. Render restarts the server with the new values.
3. Replace the same two lines in your local `.env`.
4. Check a Lobby still gets video (on the public link and locally), then delete the old key in LiveKit Cloud.

If `LIVEKIT_URL` changes (a new LiveKit project), update it in the same two places.
