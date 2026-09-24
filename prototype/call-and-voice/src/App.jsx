// PROTOTYPE — throwaway. One page: join the call, Narrator voice, Day <-> Werewolves' Turn switch.
import { useEffect, useReducer, useRef, useState } from 'react';
import { Room, RoomEvent, Track } from 'livekit-client';

const LINES = {
  wolves: "Le village s'endort. Les loups-garous se réveillent et se reconnaissent.",
  day: 'Le village se réveille.',
  button: "Le village s'endort…",
};

// ---------- log (times are relative to the last Turn message this device received) ----------
const log = [];
let marker = performance.now();
let bump = () => {};
function addLog(text) {
  const rel = Math.round(performance.now() - marker);
  log.unshift(`${new Date().toLocaleTimeString()}  +${rel}ms  ${text}`);
  if (log.length > 300) log.pop();
  bump();
}

// ---------- Narrator voice ----------
function frVoice() {
  const vs = speechSynthesis.getVoices();
  return vs.find((v) => v.lang === 'fr-FR') || vs.find((v) => v.lang?.startsWith('fr'));
}

export default function App() {
  const [, rerender] = useReducer((x) => x + 1, 0);
  bump = rerender;
  const [name, setName] = useState('');
  const [role, setRole] = useState('villager');
  const [unlockOnJoin, setUnlockOnJoin] = useState(true);
  const [muteWhileSpeaking, setMuteWhileSpeaking] = useState(true);
  const [autoSpeak, setAutoSpeak] = useState(true);
  const [turn, setTurn] = useState('day');
  const [needsAudioTap, setNeedsAudioTap] = useState(false);
  const roomRef = useRef(null);
  const opts = useRef({});
  opts.current = { muteWhileSpeaking, autoSpeak, role };

  useEffect(() => {
    const onVoices = () => { addLog(`voices loaded: fr voice = ${frVoice()?.name ?? 'NONE'}`); };
    speechSynthesis.addEventListener?.('voiceschanged', onVoices);
    return () => speechSynthesis.removeEventListener?.('voiceschanged', onVoices);
  }, []);

  async function speak(text) {
    const room = roomRef.current;
    const lp = room?.localParticipant;
    const micWasOn = lp?.isMicrophoneEnabled;
    const mute = opts.current.muteWhileSpeaking && micWasOn;
    if (mute) await lp.setMicrophoneEnabled(false);
    const u = new SpeechSynthesisUtterance(text);
    u.lang = 'fr-FR';
    const v = frVoice();
    if (v) u.voice = v;
    const t = performance.now();
    u.onstart = () => addLog(`🔊 speech START after ${Math.round(performance.now() - t)}ms (voice ${v?.name ?? 'default'}, mic muted: ${!!mute})`);
    const done = async (what) => {
      addLog(`🔊 speech ${what}`);
      if (mute) { await lp.setMicrophoneEnabled(true); addLog('mic re-enabled'); }
    };
    u.onend = () => done('END');
    u.onerror = (e) => done(`ERROR: ${e.error}`);
    speechSynthesis.cancel();
    speechSynthesis.speak(u);
    addLog(`speak() called: "${text}"`);
    // if onstart never shows in the log, the voice was blocked
  }

  // Lever 2: tell the SFU who may subscribe to MY tracks.
  function applyPublisherPermissions(nextTurn, wolves) {
    const lp = roomRef.current.localParticipant;
    if (nextTurn === 'day') {
      lp.setTrackSubscriptionPermissions(true);
      addLog('lever2: my tracks → everyone');
    } else if (opts.current.role === 'wolf') {
      const others = wolves.filter((w) => w !== lp.identity);
      lp.setTrackSubscriptionPermissions(false, others.map((id) => ({ participantIdentity: id, allowAll: true })));
      addLog(`lever2: my tracks → wolves only [${others.join(', ')}]`);
    } else {
      lp.setTrackSubscriptionPermissions(false, []);
      addLog('lever2: my tracks → nobody');
    }
  }

  async function join() {
    // Must happen synchronously inside the tap, before any await.
    if (unlockOnJoin) { speechSynthesis.speak(new SpeechSynthesisUtterance('')); addLog('voice unlocked with speak("") on Join tap'); }
    const room = new Room({ adaptiveStream: true, dynacast: true });
    roomRef.current = room;
    room.startAudio().catch(() => {});

    const who = (p) => p?.name || p?.identity || '?';
    room
      .on(RoomEvent.TrackSubscribed, (track, _pub, p) => {
        addLog(`✅ subscribed ${track.kind} of ${who(p)}`);
        if (track.kind === Track.Kind.Audio) {
          const el = track.attach();
          el.dataset.sid = track.sid;
          document.getElementById('audio-sink').appendChild(el);
        }
        rerender();
      })
      .on(RoomEvent.TrackUnsubscribed, (track, _pub, p) => {
        addLog(`⛔ unsubscribed ${track.kind} of ${who(p)}`);
        track.detach().forEach((el) => el.remove());
        rerender();
      })
      .on(RoomEvent.TrackSubscriptionPermissionChanged, (pub, status, p) => addLog(`perm of ${who(p)}'s ${pub.kind}: ${status}`))
      .on(RoomEvent.TrackSubscriptionFailed, (sid, p) => addLog(`subscription FAILED ${sid} of ${who(p)}`))
      .on(RoomEvent.ParticipantPermissionsChanged, (_prev, p) => {
        if (p === room.localParticipant) addLog(`lever1: my canSubscribe = ${p.permissions?.canSubscribe}`);
        rerender();
      })
      .on(RoomEvent.TrackStreamStateChanged, (pub, state, p) => addLog(`stream ${pub.kind} of ${who(p)}: ${state}`))
      .on(RoomEvent.ParticipantConnected, (p) => { addLog(`${who(p)} joined`); rerender(); })
      .on(RoomEvent.ParticipantDisconnected, (p) => { addLog(`${who(p)} left`); rerender(); })
      .on(RoomEvent.AudioPlaybackStatusChanged, () => { setNeedsAudioTap(!room.canPlaybackAudio); addLog(`canPlaybackAudio = ${room.canPlaybackAudio}`); })
      .on(RoomEvent.Reconnecting, () => addLog('RECONNECTING'))
      .on(RoomEvent.Reconnected, () => addLog('reconnected'))
      .on(RoomEvent.Disconnected, (r) => addLog(`DISCONNECTED ${r ?? ''}`))
      .on(RoomEvent.MediaDevicesError, (e) => addLog(`media error: ${e.message}`))
      .on(RoomEvent.DataReceived, (payload, _p, _k, topic) => {
        if (topic !== 'turn') return;
        const msg = JSON.parse(new TextDecoder().decode(payload));
        marker = performance.now();
        addLog(`━━ TURN → ${msg.turn} (wolves: ${msg.wolves.join(', ') || 'none'})`);
        setTurn(msg.turn);
        applyPublisherPermissions(msg.turn, msg.wolves);
        if (opts.current.autoSpeak) speak(LINES[msg.turn]);
      });

    const res = await fetch('/api/token', {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ name: name || 'anon', role }),
    }).then((r) => r.json());
    if (res.error) { addLog(`token error: ${res.error}`); roomRef.current = null; rerender(); return; }
    setTurn(res.turn);
    await room.connect(res.url, res.token);
    addLog(`connected as ${res.identity} (${role}), turn = ${res.turn}`);
    await room.localParticipant.enableCameraAndMicrophone().catch((e) => addLog(`cam/mic error: ${e.message}`));
    if (res.turn === 'wolves') {
      const st = await fetch('/api/state').then((r) => r.json());
      const wolves = st.participants.filter((p) => { try { return JSON.parse(p.metadata).role === 'wolf'; } catch { return false; } }).map((p) => p.identity);
      applyPublisherPermissions('wolves', wolves);
    }
    rerender();
  }

  async function switchTo(next) {
    const t = performance.now();
    addLog(`→ asked server for ${next}`);
    const r = await fetch('/api/turn', {
      method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ turn: next }),
    }).then((x) => x.json());
    addLog(`server done in ${Math.round(performance.now() - t)}ms (updateParticipant ${r.updateParticipantMs}ms, sendData ${r.sendDataMs}ms)`);
  }

  function forceResubscribe() {
    for (const p of roomRef.current.remoteParticipants.values())
      for (const pub of p.trackPublications.values()) { pub.setSubscribed(false); pub.setSubscribed(true); }
    addLog('forced resubscribe on all publications');
  }

  function copyLog() {
    const txt = `${navigator.userAgent}\n\n${[...log].reverse().join('\n')}`;
    navigator.clipboard.writeText(txt).then(() => addLog('log copied'), () => alert(txt));
  }

  const room = roomRef.current;
  if (!room) {
    return (
      <div className="join">
        <p className="proto">PROTOTYPE — call + Narrator voice</p>
        <input placeholder="Name" value={name} onChange={(e) => setName(e.target.value)} />
        <div className="row">
          <label><input type="radio" checked={role === 'villager'} onChange={() => setRole('villager')} /> Villager</label>
          <label><input type="radio" checked={role === 'wolf'} onChange={() => setRole('wolf')} /> Werewolf</label>
        </div>
        <label><input type="checkbox" checked={unlockOnJoin} onChange={(e) => setUnlockOnJoin(e.target.checked)} /> Unlock voice with speak("") on Join tap</label>
        <button className="big" onClick={join}>Join</button>
        <pre className="log">{log.join('\n')}</pre>
      </div>
    );
  }

  const lp = room.localParticipant;
  const asleep = turn === 'wolves' && role !== 'wolf';
  const remotes = [...room.remoteParticipants.values()];
  const session = navigator.audioSession;

  return (
    <div>
      <p className="proto">PROTOTYPE — {name} ({role}) — turn: <b>{turn === 'day' ? 'Day' : "Werewolves' Turn"}</b></p>
      {needsAudioTap && <button className="big" onClick={() => room.startAudio()}>Tap to enable call audio</button>}

      <div className="grid">
        {asleep && <div className="asleep">Le village dort 🌙</div>}
        <Tile participant={lp} local />
        {remotes.map((p) => <Tile key={p.identity} participant={p} />)}
      </div>

      <div className="row">
        <button onClick={() => speak(LINES.button)}>🗣 Narrator</button>
        <button onClick={() => switchTo('day')}>☀️ Day</button>
        <button onClick={() => switchTo('wolves')}>🐺 Werewolves' Turn</button>
        <button onClick={forceResubscribe}>↻ Resubscribe</button>
        <button onClick={copyLog}>📋 Copy log</button>
      </div>
      <div className="row">
        <label><input type="checkbox" checked={muteWhileSpeaking} onChange={(e) => setMuteWhileSpeaking(e.target.checked)} /> mute mic while Narrator speaks</label>
        <label><input type="checkbox" checked={autoSpeak} onChange={(e) => setAutoSpeak(e.target.checked)} /> speak on Turn change</label>
        <label><input type="checkbox" checked={lp.isMicrophoneEnabled} onChange={(e) => lp.setMicrophoneEnabled(e.target.checked).then(rerender)} /> mic</label>
        {session && (
          <label>audioSession
            <select value={session.type} onChange={(e) => { session.type = e.target.value; addLog(`audioSession.type = ${session.type}`); }}>
              {['auto', 'play-and-record', 'playback', 'ambient', 'transient', 'transient-solo'].map((t) => <option key={t}>{t}</option>)}
            </select>
          </label>
        )}
      </div>

      <pre className="state">{JSON.stringify({
        myCanSubscribe: lp.permissions?.canSubscribe,
        micEnabled: lp.isMicrophoneEnabled,
        canPlaybackAudio: room.canPlaybackAudio,
        frVoice: frVoice()?.name ?? null,
        speaking: speechSynthesis.speaking,
        audioSession: session?.type ?? 'n/a',
        remotes: remotes.map((p) => ({
          who: p.name,
          tracks: [...p.trackPublications.values()].map((t) => `${t.kind}:${t.isSubscribed ? 'sub' : 'unsub'}${t.isMuted ? '(muted)' : ''}:${t.permissionStatus ?? ''}`),
        })),
      }, null, 1)}</pre>
      <pre className="log">{log.join('\n')}</pre>
    </div>
  );
}

function Tile({ participant, local }) {
  const ref = useRef(null);
  const pub = participant.getTrackPublication(Track.Source.Camera);
  const track = pub?.track;
  useEffect(() => {
    if (!track || !ref.current) return;
    track.attach(ref.current);
    const el = ref.current;
    const t = performance.now();
    const onPlaying = () => addLog(`🎞 first frame of ${participant.name} after ${Math.round(performance.now() - t)}ms`);
    if (!local) el.addEventListener('playing', onPlaying, { once: true });
    return () => { track.detach(el); el.removeEventListener('playing', onPlaying); };
  }, [track]);
  return (
    <div className="tile">
      {track ? <video ref={ref} autoPlay playsInline muted={local} /> : <div className="black">no video</div>}
      <span>{participant.name}{local ? ' (me)' : ''}</span>
    </div>
  );
}
