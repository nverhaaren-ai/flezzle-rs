# flezzle-rs Roadmap

Deterministic, browser-playable platformer level player (eventually creator) with
first-class input-trace replay and fuzzing-style state-space exploration. Background
and sources: [ldtk-fuzzing-research-summary.md](ldtk-fuzzing-research-summary.md).

This file is a [task-dag](https://github.com/nverhaaren/task-dag) file — the tables
below are the canonical task store, and the diagram is generated from them.

<!-- task-dag:graph -->
```mermaid
graph LR
  F1["F1 Run the `bevy_ecs_ldtk` platformer example nati…"]:::pending
  F2["F2 Determinism pass: discrete tick clock (FixedUpd…"]:::pending
  F3["F3 Bare-bones WASM build playable in a browser (no…"]:::pending
  F4["F4 Verify determinism empirically: same input sequ…"]:::pending
  F5["F5 Input trace format + record/replay: save a play…"]:::pending
  F6["F6 Random input trace generator — the first fuzzin…"]:::pending
  F7["F7 Prior-art survey for exploration harness: AFL++…"]:::pending
  F8["F8 Support user-authored LDtk levels, starting wit…"]:::pending
  F9["F9 Minimal asset story for custom levels: bundled …"]:::pending
  F10["F10 IJON-style state annotations (e.g. maximize pla…"]:::pending
  F11["F11 First exploration experiment: drive the game wi…"]:::pending
  F1 --> F2
  F1 --> F3
  F2 --> F4
  F3 --> F4
  F2 --> F5
  F5 --> F6
  F4 --> F8
  F5 --> F8
  F8 --> F9
  F5 --> F10
  F8 --> F10
  F6 --> F11
  F7 --> F11
  F10 --> F11
  classDef pending fill:#3a3a3a,stroke:#666,color:#ddd;
  classDef inprogress fill:#b8860b,stroke:#daa520,color:#fff;
  classDef done fill:#1f7a3f,stroke:#2ea043,color:#fff;
  classDef blocked fill:#8b1a1a,stroke:#da3633,color:#fff;
```
<!-- /task-dag:graph -->

## Phase 1 — Deterministic playable core

| ID | Task | Depends on | Status |
|----|------|-----------|--------|
| F1 | Run the `bevy_ecs_ldtk` platformer example natively; pin Bevy / bevy_ecs_ldtk / Rapier versions | — | `[ ]` |
| F2 | Determinism pass: discrete tick clock (FixedUpdate, constant dt), explicit system ordering, no HashMap-iteration-order leaks, seeded RNG only | F1 | `[ ]` |
| F3 | Bare-bones WASM build playable in a browser (no threads, no SIMD) | F1 | `[ ]` |
| F4 | Verify determinism empirically: same input sequence → same trace, natively and in WASM; note any native-vs-WASM divergence | F2, F3 | `[ ]` |
| F5 | Input trace format + record/replay: save a play session as a trace file, replay it to an identical outcome | F2 | `[ ]` |
| F6 | Random input trace generator — the first fuzzing primitive | F5 | `[ ]` |
| F7 | Prior-art survey for exploration harness: AFL++/IJON, libafl, TAS tooling, public Antithesis-adjacent code; use with attribution where applicable | — | `[ ]` |

## Phase 2 — Custom levels

| ID | Task | Depends on | Status |
|----|------|-----------|--------|
| F8 | Support user-authored LDtk levels, starting with one simple example level; must preserve web play, determinism, and replay | F4, F5 | `[ ]` |
| F9 | Minimal asset story for custom levels: bundled default tileset/sprites; user-supplied assets later | F8 | `[ ]` |
| F10 | IJON-style state annotations (e.g. maximize player x) exposing ECS state to an exploration harness | F5, F8 | `[ ]` |
| F11 | First exploration experiment: drive the game with generated traces against an annotated level; evaluate existing tools (per F7) vs. new harness | F6, F7, F10 | `[ ]` |

## Later (unscheduled)

Not yet tasks — direction notes, roughly in order of interest:

- Exploration harness maturation; possibly surfaced in web mode as "Auto-TAS".
- Game mechanics depth: enemies, gizmos, hazards.
- In-browser level editor (LDtk remains the editor until then, possibly for a long while).
- Public hosting; a shared repository of uploaded levels.
- Linking levels into worlds; large interconnected Metroidvania-style areas with tiered gates.
- Save states (natural given determinism) and a more formal "save file" for longer play.
- Other genres (Zelda-likes, Metroidvanias) and customizable character moves (e.g. Ori-style bash).

## Log
