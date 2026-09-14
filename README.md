# flezzle-rs

Deterministic open-source Rust platformer level creator/player — long-term, a
browser-based Mario-Maker-style create/save/upload/play loop with first-class
input-trace replay, enabling fuzzing-style level exploration. See
[`ROADMAP.md`](ROADMAP.md) (task-dag) and
[`ldtk-fuzzing-research-summary.md`](ldtk-fuzzing-research-summary.md) for the
plan and the research behind it.

Current state: milestone 01 — the `bevy_ecs_ldtk` platformer example as a
library-first crate (Bevy 0.19, bevy_ecs_ldtk 0.15, Avian 0.7). On
`milestone/01-start` the two exercise stubs are unimplemented and the tests
are red *by design* — see workbook project 01. Adapted code and CC-licensed
assets are credited in [`ATTRIBUTION.md`](ATTRIBUTION.md).

## Run

```bash
# System deps (Debian/Ubuntu): pkg-config libasound2-dev libudev-dev libwayland-dev
cargo run            # dev profile is fine: deps are optimized (see Cargo.toml)
```

Controls: A/D move, W/S climb ladders, Space jump (grounded or climbing),
R restart level, P debug-print the player's inventory to the terminal.

## Test

```bash
cargo test           # headless smoke tests — no window or GPU needed
```

## Companion workbook

Hands-on projects for studying/co-developing this codebase live in
[flezzle-rs-workbook](https://github.com/nverhaaren/flezzle-rs-workbook).
