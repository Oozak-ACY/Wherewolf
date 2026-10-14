// PROTOTYPE, throwaway. Plays the Narrator's lines through ElevenLabs (via the
// dev-server proxy in vite.config.js) or the browser's speech, and logs, for
// every line, how long until sound starts, whether it came from the cache, and
// how many characters it cost.

import { SOUNDS, BEDS, mix, unlockAmbiance, setBed, cue, duck, applyMix, stopAll, isReady } from "./ambiance.js";

const $ = (id) => document.getElementById(id);
const player = new Audio();
player.preload = "auto";

// Lines copied from client/src/strings.ts (fr.narrator). "new" ones don't exist
// yet: the in-person table probably needs the Narrator to put each Role back to sleep.
const names = () => $("players").value.split(",").map((s) => s.trim()).filter(Boolean);
const FIXED = [
  "La nuit tombe. La Voyante se réveille.",
  "La Voyante se rendort.", // new
  "Les Loups-Garous se réveillent.",
  "Les Loups-Garous se rendorment.", // new
  "La Sorcière se réveille.",
  "La Sorcière se rendort.", // new
  "Le jour se lève. Personne n'est mort cette nuit.",
  "Le village débat : qui sont les Loups-Garous ?",
  "Le village élit son Maire.",
  "Le village vote.",
  "Personne n'est éliminé.",
  "Le Chasseur n'a tiré sur personne.",
  "Le Village gagne : les Loups-Garous sont morts.",
  "Les Loups-Garous gagnent : le village est à eux.",
];
const NEW = new Set(["La Voyante se rendort.", "Les Loups-Garous se rendorment.", "La Sorcière se rendort."]);
const dynamic = (n) => [
  `Le jour se lève. Cette nuit, le village a perdu ${n[1]} (Voyante).`,
  `Le jour se lève. Cette nuit, le village a perdu ${n[1]} (Voyante), ${n[6]} (Villageois).`,
  `${n[3]} est élu Maire.`,
  `Le sort désigne ${n[3]} comme Maire.`,
  `Égalité entre ${n[2]}, ${n[4]} : le Maire choisit qui est éliminé.`,
  `Le village a éliminé ${n[5]}, qui était Loup-Garou.`,
  `${n[7]}, le Chasseur, emporte quelqu'un avec lui : il choisit sur qui tirer.`,
  `Le Chasseur a tiré sur ${n[0]}, qui était Sorcière.`,
  `${n[3]}, le Maire, désigne son successeur.`,
  `${n[2]} devient Maire.`,
];
// A Night and a Day at an 8-Player table. `wait`: the Players then act on their phones.
// `bed`: the background switches before the line. `cue`: a sound plays, and the
// Narrator starts `lead` ms later, over its tail.
const script = (n) => [
  { bed: "night", text: "La nuit tombe. La Voyante se réveille.", wait: true },
  { text: "La Voyante se rendort." },
  { cue: "wolf-howl", lead: 1800, text: "Les Loups-Garous se réveillent.", wait: true },
  { text: "Les Loups-Garous se rendorment." },
  { cue: "bottle-smash", lead: 900, text: "La Sorcière se réveille.", wait: true },
  { text: "La Sorcière se rendort." },
  { bed: "day", text: `Le jour se lève. Cette nuit, le village a perdu ${n[1]} (Voyante).` },
  { text: "Le village élit son Maire.", wait: true },
  { text: `${n[3]} est élu Maire.` },
  { text: "Le village débat : qui sont les Loups-Garous ?", wait: true },
  { text: "Le village vote.", wait: true },
  { cue: "sword-slice", lead: 700, text: `Le village a éliminé ${n[5]}, qui était Chasseur.` },
  { text: `${n[5]}, le Chasseur, emporte quelqu'un avec lui : il choisit sur qui tirer.`, wait: true },
  { cue: "gunshot", lead: 900, text: `Le Chasseur a tiré sur ${n[6]}, qui était Loup-Garou.` },
  { text: "Le Village gagne : les Loups-Garous sont morts." },
];

// ---- settings ---------------------------------------------------------------

const settings = () => ({
  engine: $("engine").value,
  voice: $("voice").value,
  model: $("model").value,
  speed: $("speed").value,
  stability: $("stability").value,
});
const ttsUrl = (text, s = settings()) =>
  "/api/tts?" + new URLSearchParams({ text, voice: s.voice, model: s.model, speed: s.speed, stability: s.stability });

for (const id of ["speed", "stability", "gap"]) {
  $(id).oninput = () => ($(id + "Val").textContent = $(id).value);
}

let voices = { mine: [], shared: [] };
async function loadVoices() {
  const r = await fetch("/api/voices");
  const v = await r.json();
  if (v.error) return showError(v.error);
  voices = v;
  v.errors?.forEach((e) => showError(JSON.stringify(e)));
  const sel = $("voice");
  const keep = sel.value;
  sel.innerHTML = "";
  const group = (label, list, fmt) => {
    const g = document.createElement("optgroup");
    g.label = label;
    for (const x of list) g.append(new Option(fmt(x), x.id));
    sel.append(g);
  };
  group("My voices (premade + Voice Design: free)", v.mine, (x) => `${x.name} ${x.labels?.accent ?? ""} ${x.labels?.gender ?? ""}`);
  group("French Voice Library (paid plan or $5 credits)", v.shared, (x) => `${x.name} · ${x.accent ?? ""} ${x.gender ?? ""}`);
  if (keep && [...sel.options].some((o) => o.value === keep)) sel.value = keep;
}
const sharedVoice = () => voices.shared.find((x) => x.id === $("voice").value);

$("previewVoice").onclick = () => {
  const v = sharedVoice();
  if (!v?.preview) return showError("Previews only exist for Voice Library voices.");
  // Its own element: a cross-origin file through the Web Audio voice bus would play silent.
  new Audio(v.preview).play();
};
$("addVoice").onclick = async () => {
  const v = sharedVoice();
  if (!v) return showError("Pick a voice from the French Voice Library group.");
  const r = await fetch("/api/add-voice?" + new URLSearchParams({ owner: v.owner, id: v.id, name: `Narrateur ${v.name}` }));
  const body = await r.json();
  if (!r.ok) return showError("Add voice: " + JSON.stringify(body));
  await loadVoices();
  $("voice").value = body.voice_id;
};

// ---- unlock (iOS only lets a page play later if it played during a tap) ----

let unlocked = false;
const loadedSounds = [];
$("unlock").onclick = async () => {
  // A 0.1s silent WAV.
  const silence = "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQAAAAA=";
  player.src = silence;
  try { await player.play(); } catch (e) { showError("unlock: " + e); }
  if ("speechSynthesis" in window) speechSynthesis.speak(new SpeechSynthesisUtterance(""));
  try { await navigator.wakeLock?.request("screen"); } catch {}
  $("ambianceStatus").textContent = "Loading sounds…";
  unlockAmbiance(player, (name, seconds) => {
    loadedSounds.push(`${name} (${seconds.toFixed(1)}s)`);
    $("ambianceStatus").textContent = `Loaded ${loadedSounds.length}/${Object.keys(SOUNDS).length}: ${loadedSounds.join(", ")}`;
  }).catch((e) => showError("ambiance: " + e));
  unlocked = true;
  $("unlock").textContent = "Audio unlocked ✓ (screen kept on)";
  $("unlock").disabled = true;
};

// ---- playing a line -----------------------------------------------------------

let runId = 0; // bumped by Stop
const log = [];

function frenchVoice() {
  const fr = speechSynthesis.getVoices().filter((v) => v.lang.toLowerCase().startsWith("fr"));
  return fr.find((v) => v.lang.toLowerCase() === "fr-fr") ?? fr[0] ?? null;
}

function speakBrowser(text, t0) {
  return new Promise((resolve) => {
    const u = new SpeechSynthesisUtterance(text);
    u.lang = "fr-FR";
    u.voice = frenchVoice();
    let started;
    u.onstart = () => (started = performance.now());
    u.onend = u.onerror = (e) =>
      resolve({ latency: started ? started - t0 : null, length: started ? performance.now() - started : null, error: e.error });
    speechSynthesis.speak(u);
  });
}

function playEleven(url, t0) {
  return new Promise((resolve) => {
    let started;
    const cleanup = () => { player.onplaying = player.onended = player.onerror = null; };
    player.onplaying = () => (started ??= performance.now());
    player.onended = () => { cleanup(); resolve({ latency: started - t0, length: performance.now() - started }); };
    const fail = async (why) => {
      cleanup();
      const detail = await fetch(url).then((r) => (r.ok ? "" : r.text())).catch(() => "");
      resolve({ latency: null, length: null, error: `${why} ${detail.slice(0, 300)}` });
    };
    player.onerror = () => fail(`audio error ${player.error?.code ?? ""}`);
    player.src = url;
    player.play().catch((e) => fail(`play() refused: ${e.name}`));
  });
}

const fetched = new Set(); // URLs the server has fully generated this page load

async function say(text, how) {
  const s = settings();
  const entry = { n: log.length + 1, text, how, engine: s.engine, model: s.model };
  $("now").innerHTML = `Speaking: <b>${text}</b>`;
  duck(true);
  const t0 = performance.now();
  let result;
  if (s.engine === "browser") {
    result = await speakBrowser(text, t0);
    entry.cache = "-";
  } else {
    const url = ttsUrl(text, s);
    entry.cache = fetched.has(url) ? "hit" : "miss";
    result = await playEleven(url, t0);
    if (!result.error) fetched.add(url);
    entry.chars = entry.cache === "miss" && !result.error ? text.length : 0;
  }
  duck(false);
  Object.assign(entry, result);
  log.push(entry);
  render();
  $("now").textContent = "Idle";
  return entry;
}

async function prefetch(text) {
  if (settings().engine !== "eleven") return;
  const url = ttsUrl(text);
  if (fetched.has(url)) return;
  const r = await fetch(url);
  await r.arrayBuffer();
  if (r.ok) {
    fetched.add(url);
    log.push({ n: log.length + 1, text, how: "prefetch", engine: "eleven", model: settings().model, cache: "gen", chars: text.length });
    render();
  }
}

// ---- the controls ---------------------------------------------------------------

function renderLines() {
  const box = $("lines");
  box.innerHTML = "";
  for (const text of [...FIXED, ...dynamic(names())]) {
    const b = document.createElement("button");
    b.innerHTML = text + (NEW.has(text) ? " <small>(new, in-person)</small>" : "");
    b.onclick = () => say(text, "tap");
    box.append(b);
  }
}
$("players").onchange = renderLines;
$("sayFree").onclick = () => say($("free").value, "tap");

$("script").onclick = async () => {
  if (!unlocked) showError("Not unlocked yet: on iOS the scripted run will likely be refused. Try it anyway to see.");
  const me = ++runId;
  const steps = script(names());
  for (let i = 0; i < steps.length && me === runId; i++) {
    const next = steps[i + 1];
    if ($("prefetch").checked && next) prefetch(next.text);
    const step = steps[i];
    if ($("withAmbiance").checked) {
      if (step.bed) { bedName = step.bed; setBed(step.bed); showMix(); }
      if (step.cue) await cue(step.cue, step.lead);
    }
    await say(step.text, "script");
    if (steps[i].wait && me === runId) {
      $("now").textContent = `Players act on their phones… (${$("gap").value}s)`;
      await new Promise((r) => setTimeout(r, $("gap").value * 1000));
    }
  }
  const chars = steps.reduce((sum, s) => sum + s.text.length, 0);
  $("now").textContent = `Scripted run done: ${chars} characters of Narrator speech in this Night + Day.`;
  refreshUsage();
};
$("stop").onclick = () => {
  runId++;
  player.pause();
  speechSynthesis.cancel();
  stopAll();
  bedName = "none";
  showMix();
  $("now").textContent = "Stopped";
};
$("warm").onclick = async () => {
  for (const text of FIXED) await prefetch(text);
  refreshUsage();
};
$("clear").onclick = async () => {
  await fetch("/api/clear-cache");
  fetched.clear();
};

// ---- surfacing state ----------------------------------------------------------------

let usageAtLoad = null;
async function refreshUsage() {
  const u = await (await fetch("/api/usage")).json();
  if (u.error) return showError("usage: " + JSON.stringify(u.error));
  usageAtLoad ??= u.used;
  const sent = log.reduce((s, e) => s + (e.chars ?? 0), 0);
  $("usage").innerHTML =
    `Tier <b>${u.tier}</b>: ${u.used.toLocaleString()} / ${u.limit.toLocaleString()} credits used this month ` +
    `(resets ${new Date(u.resets * 1000).toLocaleDateString()}).<br>` +
    `Since this page loaded: <b>${(u.used - usageAtLoad).toLocaleString()}</b> credits for <b>${sent}</b> characters sent.`;
}
$("refreshUsage").onclick = refreshUsage;

const ms = (x) => (x == null ? "—" : `${Math.round(x)} ms`);
const tone = (x) => (x == null ? "bad" : x < 400 ? "ok" : x < 1000 ? "mid" : "bad");

function render() {
  $("log").innerHTML = log
    .slice()
    .reverse()
    .map(
      (e) => `<tr>
        <td>${e.n}</td>
        <td>${e.text}${e.error ? `<br><span class="bad">${e.error}</span>` : ""}</td>
        <td>${e.how}<br>${e.engine === "browser" ? "browser" : e.model.replace("eleven_", "")}<br>${e.cache}</td>
        <td class="num ${e.how === "prefetch" ? "" : tone(e.latency)}">${e.how === "prefetch" ? "" : ms(e.latency)}</td>
        <td class="num">${ms(e.length)}</td>
        <td class="num">${e.chars ?? ""}</td>
      </tr>`,
    )
    .join("");
  // Median latency per engine/model and cache state.
  const groups = {};
  for (const e of log) {
    if (e.latency == null || e.how === "prefetch") continue;
    const k = `${e.engine === "browser" ? "browser" : e.model} · ${e.cache}`;
    (groups[k] ??= []).push(e.latency);
  }
  $("summary").innerHTML = Object.entries(groups)
    .map(([k, xs]) => {
      xs.sort((a, b) => a - b);
      return `<div>${k}: median <b class="${tone(xs[xs.length >> 1])}">${ms(xs[xs.length >> 1])}</b>, worst ${ms(xs.at(-1))} (${xs.length} lines)</div>`;
    })
    .join("");
}

function showError(msg) {
  $("errors").textContent = `${new Date().toLocaleTimeString()} ${msg}\n` + $("errors").textContent;
}

renderLines();
loadVoices().catch((e) => showError(String(e)));
refreshUsage().catch((e) => showError(String(e)));

// ---- ambiance controls ------------------------------------------------------------

let bedName = "none";
function showMix() {
  $("mixState").textContent = JSON.stringify({ background: bedName, ...mix });
}

for (const bed of Object.keys(BEDS)) {
  const b = document.createElement("button");
  b.className = "secondary";
  b.textContent = bed === "none" ? "No background" : `${bed} background`;
  b.onclick = () => {
    if (!isReady()) return showError("Tap Unlock audio first and wait for the sounds to load.");
    bedName = bed;
    setBed(bed);
    showMix();
  };
  $("beds").append(b);
}
for (const [name, label] of Object.entries(SOUNDS)) {
  if (Object.values(BEDS).flat().includes(name)) continue;
  const b = document.createElement("button");
  b.className = "secondary";
  b.textContent = "▶ " + label;
  b.onclick = () => (isReady() ? cue(name) : showError("Tap Unlock audio first and wait for the sounds to load."));
  $("sfx").append(b);
}
// What the Host would get in the real game: three faders, 0 to 150%.
const faders = [
  ["Narrator", () => mix.voice, (v) => (mix.voice = v)],
  ["Music & ambiance", () => mix.bed, (v) => (mix.bed = v)],
  ["Sound effects", () => mix.sfx, (v) => (mix.sfx = v)],
];
for (const [label, get, set] of faders) {
  const l = document.createElement("label");
  l.className = "fader";
  const pct = (v) => `${Math.round(v * 100)}%`;
  l.innerHTML = `<span class="name">${label}</span><input type="range" min="0" max="1.5" step="0.05" value="${get()}" /><span class="pct">${pct(get())}</span>`;
  const input = l.querySelector("input");
  input.oninput = () => {
    set(+input.value);
    l.querySelector(".pct").textContent = pct(+input.value);
    applyMix();
    showMix();
  };
  $("faders").append(l);
}

// Prototype-only tuning: what we'd fix in code once it sounds right.
const sliders = [
  ["· night music layer", () => mix.layer["night-music"], (v) => (mix.layer["night-music"] = v)],
  ["· night wind layer", () => mix.layer["night-wind"], (v) => (mix.layer["night-wind"] = v)],
  ["· tavern layer", () => mix.layer["day-tavern"], (v) => (mix.layer["day-tavern"] = v)],
  ["Background level while the Narrator speaks", () => mix.duck, (v) => (mix.duck = v)],
  ["Crossfade between Night and Day (s)", () => mix.fade, (v) => (mix.fade = v), 8],
];
for (const [label, get, set, max = 1] of sliders) {
  const l = document.createElement("label");
  l.innerHTML = `${label}: <span>${get()}</span><input type="range" min="0" max="${max}" step="${max > 1 ? 0.5 : 0.05}" value="${get()}" />`;
  const input = l.querySelector("input");
  input.oninput = () => {
    set(+input.value);
    l.querySelector("span").textContent = input.value;
    applyMix();
    showMix();
  };
  $("mixer").append(l);
}
showMix();
