// PROTOTYPE — throwaway. Answers the two questions in docs/handoffs/prototype-call-and-voice.md.
// The "game server" is a Vite middleware (Node, not Rust) so one command serves page + API
// behind one HTTPS tunnel. The question is about phones, not the server language.
import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';
import { appendFileSync } from 'node:fs';
import { AccessToken, RoomServiceClient, DataPacket_Kind } from 'livekit-server-sdk';

const ROOM = 'wherewolf-proto';
const LOG_FILE = 'PROTOTYPE-session-logs.txt'; // git-ignored, wipe me

function protoServer(env) {
  const { LIVEKIT_URL, LIVEKIT_API_KEY, LIVEKIT_API_SECRET } = env;
  const httpUrl = (LIVEKIT_URL || '').replace(/^ws/, 'http');
  const rooms = new RoomServiceClient(httpUrl, LIVEKIT_API_KEY, LIVEKIT_API_SECRET);
  let turn = 'day'; // 'day' | 'wolves' — in memory only

  const perm = (canSubscribe) => ({
    // permissions are replaced atomically: always send the full set, and never touch canPublish
    canSubscribe, canPublish: true, canPublishData: true, canUpdateMetadata: true,
  });

  const json = (res, code, body) => {
    res.statusCode = code;
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify(body));
  };
  const readBody = (req) => new Promise((ok) => {
    let s = ''; req.on('data', (c) => (s += c)); req.on('end', () => ok(s ? JSON.parse(s) : {}));
  });

  async function switchTurn(next) {
    const t0 = Date.now();
    turn = next;
    const people = await rooms.listParticipants(ROOM);
    const roleOf = (p) => { try { return JSON.parse(p.metadata).role; } catch { return 'villager'; } };
    const wolves = people.filter((p) => roleOf(p) === 'wolf').map((p) => p.identity);
    const timings = [];
    // Lever 1 (hard, server-side): canSubscribe off for everyone who must perceive nobody.
    await Promise.all(people.map(async (p) => {
      const s = Date.now();
      const canSubscribe = next === 'day' || roleOf(p) === 'wolf';
      await rooms.updateParticipant(ROOM, p.identity, { permission: perm(canSubscribe) });
      timings.push({ identity: p.identity, canSubscribe, ms: Date.now() - s });
    }));
    const tLevers = Date.now();
    // Lever 2 instruction: tell every client who may subscribe to its own tracks.
    const msg = { type: 'turn', turn: next, wolves, serverT: Date.now() };
    await rooms.sendData(ROOM, new TextEncoder().encode(JSON.stringify(msg)), DataPacket_Kind.RELIABLE, { topic: 'turn' });
    const report = { turn: next, wolves, updateParticipantMs: tLevers - t0, sendDataMs: Date.now() - tLevers, timings };
    console.log('[proto] switch', JSON.stringify(report));
    return report;
  }

  return {
    name: 'proto-server',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        try {
          if (req.url === '/api/token' && req.method === 'POST') {
            if (!LIVEKIT_API_KEY) return json(res, 500, { error: 'Missing LIVEKIT_* in prototype/call-and-voice/.env' });
            const { name, role } = await readBody(req);
            const identity = `${name}-${Math.random().toString(36).slice(2, 6)}`;
            const at = new AccessToken(LIVEKIT_API_KEY, LIVEKIT_API_SECRET, {
              identity, name, metadata: JSON.stringify({ role }), ttl: '6h',
            });
            // late joiner during the Werewolves' Turn: a sleeping villager starts with canSubscribe off
            const canSubscribe = turn === 'day' || role === 'wolf';
            at.addGrant({ room: ROOM, roomJoin: true, canPublish: true, canPublishData: true, canSubscribe });
            return json(res, 200, { token: await at.toJwt(), url: LIVEKIT_URL, identity, turn });
          }
          if (req.url === '/api/turn' && req.method === 'POST') {
            const { turn: next } = await readBody(req);
            return json(res, 200, await switchTurn(next));
          }
          if (req.url === '/api/log' && req.method === 'POST') {
            // every device ships its log lines here so they can be read after the session
            const { who, ua, lines } = await readBody(req);
            const out = lines.map((l) => `${new Date().toISOString()} [${who}] ${l}`).join('\n') + '\n';
            if (lines.some((l) => l.includes('connected as'))) appendFileSync(LOG_FILE, `${new Date().toISOString()} [${who}] UA: ${ua}\n`);
            appendFileSync(LOG_FILE, out);
            return json(res, 200, {});
          }
          if (req.url === '/api/state') {
            const people = await rooms.listParticipants(ROOM).catch(() => []);
            return json(res, 200, {
              turn,
              participants: people.map((p) => ({ identity: p.identity, metadata: p.metadata, permission: p.permission })),
            });
          }
        } catch (e) {
          console.error('[proto]', e);
          return json(res, 500, { error: String(e?.message || e) });
        }
        next();
      });
    },
  };
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  return {
    plugins: [react(), protoServer(env)],
    server: { host: true, port: 5173, allowedHosts: true },
  };
});
