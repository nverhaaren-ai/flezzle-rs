# Browser property tests (Bombadil)

[Bombadil](https://github.com/antithesishq/bombadil) explores the web build
with random actions and checks *properties* over the sequence of states it
observes. The game makes its WASM-resident state observable by publishing a
per-frame snapshot (`src/debug.rs`) as `window.flezzle` and as JSON in a
hidden `<script id="flezzle-state">` element; `flezzle.spec.ts` reads that.

## Run

```bash
# once
npm install                      # types + a copy of the binary under node_modules/.bin
# or put the prebuilt `bombadil` binary on PATH: see the Bombadil manual
trunk build --cargo-profile wasm-release          # from the repo root

# then, from the repo root:
CHROME=/path/to/chrome tests/bombadil/run.sh 2m   # all bundled levels
CHROME=/path/to/chrome tests/bombadil/run.sh 90s levels/template.ldtk
```

`CHROME` is needed when no `google-chrome`/`chromium` is on `PATH`. On a
machine without a GPU (CI, sandboxes) point it at a wrapper that adds
`--use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader`.
Results land in `target/bombadil/<level>/` (a `trace.jsonl` plus a screenshot
per state); `bombadil browser inspect target/bombadil/<level>` opens a viewer,
`--reproduce` replays a run.

## Properties

| Export | Checks | Origin |
|---|---|---|
| Bombadil defaults | no uncaught exceptions, promise rejections, console errors, HTTP 4xx/5xx | stock |
| `gameBoots` | a level spawns and a player exists within 30 s | sanity |
| `ticksMonotonic`, `simulationAdvances` | tick count never regresses; keeps growing | liveness |
| `playerDrawnAboveChest` | `player.z > chest.z` whenever both exist | playtest issue 4 |
| `smoothWalking` | rendered walking speed within 35 % of simulated speed on every frame (in-page rolling metric, normalized by real frame time) | playtest issue 1 |
| `restartReloadsLevel` | holding Restart is followed by a level despawn within 3 s | playtest issue 2 |
| `pushScenarioIsReached` | the random walk gets the player up against the chest within 90 s (reachability, so the next property isn't vacuous) | method |
| `chestCanBePushed` | pushing against the chest moves it > 2 px within 3 s | playtest issue 3 |

Actions: hold A/D for 0.7–1.5 s (a custom action dispatching `keydown`/`keyup`
on the canvas — `PressKey` is only a tap), taps of Space/W/S/R.

## Gotchas learned the hard way

- Bombadil instruments JavaScript for coverage, which invalidates
  subresource-integrity hashes. The Trunk link therefore carries
  `data-integrity="none"`; with SRI on, the game silently never boots.
- Bombadil samples state only after actions, not per frame. Anything
  per-frame (smoothness) has to be aggregated *inside* the game and exposed
  as a number.
- Random exploration rarely constructs a specific scenario (standing against
  the chest). Pair each scenario property with a reachability property, and
  run on a level where the scenario is easy (`template.ldtk` for the chest).
