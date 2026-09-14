// Bombadil specification for the flezzle-rs web build.
//
// Run (from the repo root, with the bundle served on :8765):
//   CHROME=/path/to/chrome bombadil browser test --headless --time-limit=2m \
//     http://127.0.0.1:8765/ tests/bombadil/flezzle.spec.ts
//
// The game publishes a per-frame snapshot of its (WASM-resident) state as
// `window.flezzle` — see src/debug.rs. Everything below observes that object;
// Bombadil never has to look at the canvas.

import { always, eventually, now } from "@antithesishq/bombadil";
import {
  actions,
  extract,
  registerCustomAction,
} from "@antithesishq/bombadil/browser";
// Bombadil's stock properties: uncaught exceptions, promise rejections,
// error-level console logs, HTTP 4xx/5xx, and so on.
export * from "@antithesishq/bombadil/browser/defaults/properties";

// ---------------------------------------------------------------- snapshot

type Body = { pos: { x: number; y: number }; z: number; vel: { x: number; y: number } };
type Snapshot = {
  frame: number;
  tick: number;
  ticks_this_frame: number;
  zero_tick_frames: number;
  multi_tick_frames: number;
  held_actions: number;
  level: string;
  level_selection: string;
  level_spawns: number;
  level_despawns: number;
  physics_paused: boolean;
  player: (Body & { on_ground: boolean; climbing: boolean }) | null;
  chest: Body | null;
  mobs: Body[];
  camera: { x: number; y: number } | null;
  max_walk_step: number;
  max_walk_speed_error: number;
  level_bounds: { x: number; y: number; w: number; h: number } | null;
};

// The game publishes its snapshot two ways: as `window.flezzle` and as JSON
// text in `<script id="flezzle-state">`. Bombadil's extractors run in the
// page's main world, so either works; prefer the DOM copy because it is
// visible from any JS world (extensions, isolated-world drivers) and is a
// plain string snapshot rather than a live object.
function readSnapshot(document: Document, window: Window): Snapshot | null {
  const text = document.getElementById("flezzle-state")?.textContent;
  if (text) {
    try {
      return JSON.parse(text) as Snapshot;
    } catch {
      /* fall through */
    }
  }
  return (window as unknown as { flezzle?: Snapshot }).flezzle ?? null;
}

const snap = extract((state) => readSnapshot(state.document, state.window));
const snapSource = extract((state) =>
  state.document.getElementById("flezzle-state") ? "dom" : (state.window as unknown as { flezzle?: unknown }).flezzle ? "window" : "none",
);

// Bit layout of held_actions mirrors flezzle_rs::input::Action.
const ACTION = { Left: 1, Right: 2, Up: 4, Down: 8, Jump: 16, Restart: 32 } as const;
const holding = (mask: number) => ((snap.current?.held_actions ?? 0) & mask) !== 0;


// -------------------------------------------------------------- properties

/** The game booted: a snapshot exists and a level has spawned. */
export const gameBoots = eventually(
  () => (snap.current?.level_spawns ?? 0) > 0 && snap.current?.player != null,
).within(30, "seconds");

/** Ticks never go backwards between captured states (monotonic clock). */
const lastTick = extract((state) => readSnapshot(state.document, state.window)?.tick ?? 0);
let prevTick = 0;
export const ticksMonotonic = always(() => {
  const t = lastTick.current;
  const ok = t >= prevTick;
  prevTick = t;
  return ok;
});

/** Simulation never stalls: from any state, the tick count grows again soon. */
let tickAtCheck = 0;
export const simulationAdvances = always(() =>
  now(() => {
    tickAtCheck = lastTick.current;
    return snap.current?.player != null;
  }).implies(eventually(() => lastTick.current > tickAtCheck).within(5, "seconds")),
);

/**
 * Issue 4 (draw order): the player is never drawn behind the chest.
 * Reported: correct for a fraction of a second, then flips.
 */
export const playerDrawnAboveChest = always(() => {
  const s = snap.current;
  if (!s?.player || !s.chest) return true;
  return s.player.z > s.chest.z;
});

/**
 * Issue 1 (visual skips): the *rendered* walking speed stays close to the
 * simulated one on every frame. Without render interpolation, a 60 Hz tick
 * drawn at a drifting refresh rate alternates 0-tick and 2-tick frames, so
 * per-frame speed swings between 0 and 2x — the visible "jump". The error
 * is normalized by real frame time, so a slow renderer that runs many ticks
 * per frame (software GL in CI) is *not* penalized: motion per wall-second
 * is still right. Accumulated in-page over the last 120 frames because
 * Bombadil samples state far less often than the game renders.
 */
export const smoothWalking = always(() => {
  const s = snap.current;
  if (!s?.player || typeof s.max_walk_speed_error !== "number") return true;
  return s.max_walk_speed_error <= 0.35;
});

/**
 * Issue 2 (R does nothing): after a restart is requested, the level reloads
 * within a couple of seconds. (What "restart" should do to the player is a
 * separate design question; this checks the mechanism fires at all.)
 */
const despawns = extract((state) => readSnapshot(state.document, state.window)?.level_despawns ?? 0);
// Baseline = the despawn count in the most recent state where Restart was
// NOT held. (With a short tap the reload can complete before Bombadil captures
// the state in which the key shows as held, so "count at that state" would
// already include the effect and the property would wrongly fail.)
let despawnsBeforeRestart = 0;
export const restartReloadsLevel = always(() =>
  now(() => {
    if (!holding(ACTION.Restart)) {
      despawnsBeforeRestart = despawns.current;
      return false;
    }
    return true;
  }).implies(eventually(() => despawns.current > despawnsBeforeRestart).within(3, "seconds")),
);

/**
 * Reachability ("sometimes" in Antithesis terms): the random walk actually
 * gets the player up against the chest at some point, otherwise the push
 * property below is vacuously satisfied. Run on a level where the chest is
 * reachable (e.g. `?level=levels/template.ldtk`).
 */
const onTemplateLevel = extract((state) =>
  new URLSearchParams(state.window.location.search).get("level") === "levels/template.ldtk",
);
export const pushScenarioIsReached = eventually(
  () => !onTemplateLevel.current || pushingChest(),
).within(60, "seconds");

function pushingChest(): boolean {
  const s = snap.current;
  if (!s?.player || !s.chest || Math.abs(s.player.pos.y - s.chest.pos.y) > 12) return false;
  const dx = s.chest.pos.x - s.player.pos.x;
  return dx > 0 && dx < 26 && holding(ACTION.Right) && s.player.on_ground;
}

/**
 * Issue 3 (chest feels slow): pushing the chest moves it. Sanity floor, not a
 * tuning target — a chest that never moves is a bug, a slow one is a choice.
 */
const chestX = extract((state) => readSnapshot(state.document, state.window)?.chest?.pos.x ?? null);
let chestXAtPush = 0;
export const chestCanBePushed = always(() =>
  now(() => {
    const pushing = pushingChest();
    if (pushing) chestXAtPush = snap.current!.chest!.pos.x;
    return pushing;
  }).implies(
    eventually(() => (chestX.current ?? -Infinity) > chestXAtPush + 2).within(3, "seconds"),
  ),
);

/**
 * Found by the fuzzer, not by the playtest: on a level without edge walls the
 * player can walk off the side and fall forever (x reached 3078 on a 384 px
 * level). The player must stay inside the selected level, with a margin for
 * a body half-width and for neighbour transitions in multi-level worlds.
 */
export const playerStaysInLevel = always(() => {
  const s = snap.current;
  if (!s?.player || !s.level_bounds) return true;
  const b = s.level_bounds;
  const m = 64; // px of slack: half a body, plus stepping into a neighbour level
  const p = s.player.pos;
  return p.x >= b.x - m && p.x <= b.x + b.w + m && p.y >= b.y - m && p.y <= b.y + b.h + m;
});

// ----------------------------------------------------------------- actions

// DOM keyCode for Bombadil's built-in PressKey (a tap: keydown+keyup).
const KEY = { Space: 32 } as const;

/** Hold a key for `ms` milliseconds via synthetic DOM events on the canvas. */
const holdKey = registerCustomAction(
  "holdKey",
  async (document, _window, code: string, key: string, ms: number) => {
    const canvas = document.getElementById("flezzle-canvas");
    if (!canvas) throw new Error("no game canvas");
    canvas.focus();
    const ev = (type: string) =>
      new KeyboardEvent(type, { code, key, bubbles: true, cancelable: true });
    canvas.dispatchEvent(ev("keydown"));
    await new Promise((r) => setTimeout(r, ms));
    canvas.dispatchEvent(ev("keyup"));
  },
);

/** Tap a key (keydown then keyup on the next task) via synthetic DOM events. */
const tapKey = registerCustomAction("tapKey", async (document, _window, code: string, key: string) => {
  const canvas = document.getElementById("flezzle-canvas");
  if (!canvas) throw new Error("no game canvas");
  canvas.focus();
  const ev = (type: string) => new KeyboardEvent(type, { code, key, bubbles: true, cancelable: true });
  canvas.dispatchEvent(ev("keydown"));
  await new Promise((r) => setTimeout(r, 40));
  canvas.dispatchEvent(ev("keyup"));
});

const focusCanvas = registerCustomAction("focusCanvas", async (document) => {
  document.getElementById("flezzle-canvas")?.focus();
});

export const play = actions(() => {
  if (!snap.current?.player) return [focusCanvas()];
  return [
    holdKey("KeyD", "d", 700),
    holdKey("KeyA", "a", 700),
    holdKey("KeyD", "d", 1500),
    // Taps as synthetic events too: Bombadil's built-in PressKey (used for
    // Space below) is kept for comparison — see the scouting notes.
    tapKey("Space", " "),
    tapKey("KeyW", "w"),
    tapKey("KeyS", "s"),
    tapKey("KeyR", "r"),
    { PressKey: { code: KEY.Space } },
  ];
});
