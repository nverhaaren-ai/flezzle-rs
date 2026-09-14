# flezzle-rs

Deterministic open-source Rust platformer level creator/player — long-term, a
browser-based Mario-Maker-style create/save/upload/play loop with first-class
input-trace replay, enabling fuzzing-style level exploration. See
[`ROADMAP.md`](ROADMAP.md) (task-dag) and
[`ldtk-fuzzing-research-summary.md`](ldtk-fuzzing-research-summary.md) for the
plan and the research behind it.

Current state: the `bevy_ecs_ldtk` platformer example as a library-first
crate (Bevy 0.19, bevy_ecs_ldtk 0.15, Avian 0.7) with a 60 Hz tick-driven
simulation, a browser build, and user-authored LDtk levels. On
`milestone/01-start` the two workbook exercise stubs are unimplemented and
the tests are red *by design* — see workbook project 01. Adapted code and
CC-licensed assets are credited in [`ATTRIBUTION.md`](ATTRIBUTION.md).

## Run

```bash
# System deps (Debian/Ubuntu): pkg-config libasound2-dev libudev-dev libwayland-dev
cargo run                               # default level (dev profile is fine: deps are optimized)
cargo run -- levels/first-steps.ldtk    # any level under assets/
```

Controls: A/D move, W/S climb ladders, Space jump (grounded or climbing),
R restart level, P debug-print the player's inventory to the terminal.

## Play in a browser

`trunk build --cargo-profile wasm-release` produces a static bundle in
`dist/`; serve it with any static file server. Setup and details:
[`docs/web-build.md`](docs/web-build.md).

## Make a level

Levels are [LDtk](https://ldtk.io/) 1.5.3 projects. Copy
`assets/levels/template.ldtk`, draw, save, play — or write one as ASCII art
and run `tools/ldtk_gen.py`. The contract between editor and runner is in
[`docs/making-levels.md`](docs/making-levels.md).

## Test

```bash
cargo test           # headless tests: no window or GPU needed
```

Property-based tests of the *browser* build (Bombadil, random exploration
against invariants) live in [`tests/bombadil/`](tests/bombadil/README.md).
The game publishes a per-frame state snapshot for them (`src/debug.rs`).

## Companion workbook

Hands-on projects for studying/co-developing this codebase live in
[flezzle-rs-workbook](https://github.com/nverhaaren/flezzle-rs-workbook).
