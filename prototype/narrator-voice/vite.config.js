// PROTOTYPE, throwaway. A dev-server proxy to ElevenLabs so the API key never
// reaches the phone. Audio is cached in memory only (wiped on restart).
import { Readable } from "node:stream";
import { defineConfig, loadEnv } from "vite";

const API = "https://api.elevenlabs.io";

export default defineConfig(({ mode }) => {
  const key = loadEnv(mode, process.cwd(), "").ELEVENLABS_API_KEY;
  const cache = new Map(); // cacheKey -> Buffer

  const eleven = (path, init = {}) =>
    fetch(API + path, { ...init, headers: { "xi-api-key": key, ...init.headers } });

  const json = (res, status, body) => {
    res.statusCode = status;
    res.setHeader("content-type", "application/json");
    res.end(JSON.stringify(body));
  };

  const proxy = {
    name: "elevenlabs-proxy",
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        const url = new URL(req.url, "http://x");
        if (!url.pathname.startsWith("/api/")) return next();
        if (!key) return json(res, 500, { error: "ELEVENLABS_API_KEY missing from prototype/narrator-voice/.env" });
        try {
          if (url.pathname === "/api/voices") {
            const mine = await (await eleven("/v1/voices")).json();
            const shared = await (
              await eleven("/v1/shared-voices?language=fr&page_size=40&sort=usage_character_count_1y")
            ).json();
            return json(res, 200, {
              mine: (mine.voices ?? []).map((v) => ({ id: v.voice_id, name: v.name, labels: v.labels })),
              shared: (shared.voices ?? []).map((v) => ({
                id: v.voice_id,
                owner: v.public_owner_id,
                name: v.name,
                accent: v.accent,
                gender: v.gender,
                preview: v.preview_url,
              })),
              errors: [mine.detail, shared.detail].filter(Boolean),
            });
          }

          if (url.pathname === "/api/add-voice") {
            const owner = url.searchParams.get("owner");
            const id = url.searchParams.get("id");
            const r = await eleven(`/v1/voices/add/${owner}/${id}`, {
              method: "POST",
              headers: { "content-type": "application/json" },
              body: JSON.stringify({ new_name: url.searchParams.get("name") ?? "Narrateur" }),
            });
            return json(res, r.status, await r.json());
          }

          if (url.pathname === "/api/usage") {
            const r = await eleven("/v1/user/subscription");
            const s = await r.json();
            return json(res, r.status, {
              tier: s.tier,
              used: s.character_count,
              limit: s.character_limit,
              resets: s.next_character_count_reset_unix,
              error: s.detail,
            });
          }

          if (url.pathname === "/api/tts") {
            const p = Object.fromEntries(url.searchParams);
            const cacheKey = JSON.stringify([p.text, p.voice, p.model, p.speed, p.stability]);
            res.setHeader("content-type", "audio/mpeg");
            res.setHeader("cache-control", "no-store");
            const hit = cache.get(cacheKey);
            if (hit) {
              console.log(`[tts] HIT  ${p.text}`);
              return res.end(hit);
            }
            const started = Date.now();
            const body = {
              text: p.text,
              model_id: p.model,
              voice_settings: { stability: +p.stability, similarity_boost: 0.75, speed: +p.speed },
            };
            if (p.model.includes("flash") || p.model.includes("turbo")) body.language_code = "fr";
            const r = await eleven(`/v1/text-to-speech/${p.voice}/stream?output_format=mp3_44100_128`, {
              method: "POST",
              headers: { "content-type": "application/json" },
              body: JSON.stringify(body),
            });
            if (!r.ok) {
              const text = await r.text();
              console.log(`[tts] ERR ${r.status} ${text}`);
              res.setHeader("content-type", "application/json");
              res.statusCode = r.status;
              return res.end(text);
            }
            const chunks = [];
            let first = true;
            const upstream = Readable.fromWeb(r.body);
            upstream.on("data", (c) => {
              if (first) {
                console.log(`[tts] MISS ${Date.now() - started}ms to first byte from ElevenLabs: ${p.text}`);
                first = false;
              }
              chunks.push(c);
              res.write(c);
            });
            upstream.on("end", () => {
              cache.set(cacheKey, Buffer.concat(chunks));
              res.end();
            });
            upstream.on("error", (e) => res.destroy(e));
            return;
          }

          if (url.pathname === "/api/clear-cache") {
            cache.clear();
            return json(res, 200, { cleared: true });
          }
          next();
        } catch (e) {
          json(res, 500, { error: String(e) });
        }
      });
    },
  };

  return { plugins: [proxy] };
});
