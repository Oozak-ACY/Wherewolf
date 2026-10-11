# Deploying Wherewolf

Three pieces, each on its own host:

| Piece | Host | Config |
| --- | --- | --- |
| Client (Vite + React) | Vercel, project root `client/` | `client/vercel.json`, env var `VITE_SERVER_URL` |
| Game server (Rust) | Render, free web service `wherewolf-server` | `render.yaml`, env vars `LIVEKIT_*` |
| Video call | LiveKit Cloud | Keys live only on Render (and in your git-ignored `.env`) |

The client only knows the server's URL. The server is the only piece holding the LiveKit keys and hands each Player a call ticket (ADR 0001).

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

## Rotating the LiveKit keys

Do this if a secret leaks, or on a schedule. The same key is used locally and in production, so update both.

1. LiveKit Cloud → your project → Settings → API keys → create a new key. Copy its key and secret.
2. Render → wherewolf-server → Environment: replace `LIVEKIT_API_KEY` and `LIVEKIT_API_SECRET`, then Save. Render restarts the server with the new values.
3. Replace the same two lines in your local `.env`.
4. Check a Lobby still gets video (on the public link and locally), then delete the old key in LiveKit Cloud.

If `LIVEKIT_URL` changes (a new LiveKit project), update it in the same two places.
