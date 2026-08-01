# Fuzzing Kids' LDtk Platformer Levels: Research Summary

## Wider Context & Motivation

Coverage-guided fuzzers like AFL are excellent at finding bugs in parsers and protocol implementations, but they struggle with games and simulations because **code coverage is a poor proxy for progress**. A fuzzer can hit the same lines of code over and over while the player character is stuck in completely different, unproductive states — coverage looks "interesting" but nothing meaningful is happening.

**IJON** (RUB-SysSec, IEEE S&P 2020, [github.com/RUB-SysSec/ijon](https://github.com/RUB-SysSec/ijon)) addresses this by letting a developer annotate a program with lightweight hints that expose domain-specific state directly to the fuzzer, supplementing raw code coverage. The canonical example is Super Mario Bros.: a single annotation, `IJON_MAX(player_x)`, tells the fuzzer to search for inputs that push Mario's x-coordinate further right. Because SMB is a side-scroller, "maximize x" is a strong proxy for "make progress through the level," and this simple hint let AFL clear obstacles (including defeating Bowser in World 3-4) that plain coverage-guided fuzzing couldn't solve. The technique has since been folded into **AFL++**, the actively maintained AFL fork.

This connects directly to Antithesis's approach to autonomous testing (see Will Wilson's talk *"Testing a Single-Node, Single-Threaded, Distributed System Written in 1985,"* which also uses Mario as an illustrative target, and the accompanying blog post *"Why Antithesis Works"*). Both IJON's annotations and Antithesis's SDK-level assertions are ways of telling an automated exploration system *what "interesting" or "correct" means* for a system that doesn't expose that information for free. Antithesis itself keeps public reference projects along similar lines — `aardvark-arena` (a turn-based AI-vs-AI game environment built to demonstrate state-space exploration) and `glitch-grid` (a toy distributed system demonstrating fault-injection/assertion integration).

**The project idea:** let the kids design levels in a real, kid-friendly level editor (LDtk), run those levels in an open, instrumentable game engine (Bevy), and eventually add IJON-style state annotations so a fuzzer can explore the space of inputs that complete (or break) a level — a small, personal, hands-on version of the state-space-exploration idea explored in the reading list.

Note on proprietary alternatives: actual Super Mario Maker levels weren't a viable target (Nintendo's platform, closed and copyrighted), and even NES ROM + emulator approaches (which is what the IJON paper itself used) carry ROM copyright baggage. LDtk + an open engine sidesteps this entirely.

---

## Building the Game: What's Needed

### Level editor: LDtk

- **What it is:** LDtk (Level Designer Toolkit) is an open-source (MIT), free, cross-platform 2D level editor built by the director of *Dead Cells*, focused on usability for platformers and top-down games.
- **Why it fits:** Kid-friendly UI, and it outputs a well-documented JSON format that's straightforward to parse in any engine. It's under active development and open source, hosted at [github.com/deepnight/ldtk](https://github.com/deepnight/ldtk).
- **Site/docs:** [ldtk.io](https://ldtk.io/)

### Runtime: Bevy + `bevy_ecs_ldtk` + Rapier

- **`bevy_ecs_ldtk`** ([github.com/Trouv/bevy_ecs_ldtk](https://github.com/Trouv/bevy_ecs_ldtk)) is an ECS-friendly plugin that loads LDtk projects as Bevy assets, spawns levels, and lets you attach Bevy components/bundles to LDtk entities and tiles via derive macros. It supports all LDtk layer types, external levels, and hot reloading.
- It ships with a **working platformer example** runnable immediately via `cargo run --example platformer --release`, using Rapier for physics — this is the agreed starting point before touching any fuzzing infrastructure.
- **Why Bevy specifically helps the fuzzing goal:** Bevy's ECS means game state is already structured as queryable components rather than ad-hoc variables, which makes it much easier to expose specific state (player position, velocity, etc.) to an IJON-style annotation later.

### Determinism requirements

Antithesis-style reproducibility concerns apply here too, but in a narrower, more tractable form — this is a single-process synchronous game loop, not a distributed system, so no hypervisor-level control is needed. What *is* needed is the same discipline as tool-assisted-speedrun (TAS) tooling: a fixed input sequence must always produce the same trace.

Concrete points, from the research:

1. **Fixed timestep.** Use Bevy's `FixedUpdate` schedule with a constant `dt`; never integrate physics/movement against wall-clock delta time, or results will vary with frame rate/machine speed.
2. **Rapier determinism scope.** Rapier's physics results are deterministic across runs on the *same* build/platform, but not guaranteed bit-identical across different CPU architectures or compiler settings (FMA/SIMD differences) — fine for a single dev machine, but don't assume cross-machine replay works without verification.
3. **Rust `HashMap` iteration order.** Randomized per-process by default (SipHash-seeded). If any game logic's outcome depends on iteration order over a `HashMap`, that's a hidden nondeterminism source — use `BTreeMap`/`IndexMap` or a fixed-seed hasher wherever order could leak into behavior.
4. **Bevy system scheduling.** Systems can run in parallel; two systems mutating overlapping state without explicit ordering constraints can produce run-to-run differences even with identical input. Bevy's ambiguity detection can catch this during development.
5. **No unseeded RNG.** Any randomness (AI behavior, gameplay-affecting particle logic) needs an explicit, logged seed.

### Portability

| Target | Status |
|---|---|
| Desktop (Linux/macOS/Windows) | Native default target; where fuzzing work would happen |
| Web (WASM) | Supported by both Bevy and `bevy_ecs_ldtk` (which has a dedicated "atlas" feature for Wasm) |
| Mobile (iOS/Android) | Supported by Bevy, less battle-tested — more tooling/input friction than outright incompatibility |

**WASM determinism — a genuine surprise finding:** WebAssembly's spec is *stricter* about float determinism than native code. Per the WASM spec, any floating-point operation whose inputs and output are non-NaN is deterministic and agrees across all conformant engines/platforms — this sidesteps the x87-excess-precision and FMA-contraction issues that make native floating point implementation-dependent across compilers/CPUs.

Caveats surfaced in the spec and by other projects building deterministic WASM systems (e.g. the Internet Computer's canister runtime, which disables threads and SIMD and canonicalizes NaNs specifically for consensus-grade determinism):

- **NaN payload/sign is explicitly allowed to vary** between implementations — the spec defines an *allowed set* of outputs for NaN-producing operations, not one exact value. Physics engines can produce NaNs internally (degenerate/edge cases), so this is the main practical risk.
- **SIMD** determinism is less airtight than scalar float ops; some deterministic-execution runtimes disable it outright.
- **Threads** (shared memory + atomics) reintroduce real scheduling nondeterminism, just as in native code — a single-threaded WASM build is the safe choice.

Practical implication: if the WASM build avoids threads and SIMD, and gameplay logic guards against NaN propagation, the WASM build could plausibly be *more* portable/deterministic across machines than the native build — potentially a better canonical fuzzing/replay target than initially assumed. This is worth verifying empirically (e.g. via the public test harness at `wasm-float-determinism-test.pages.dev`) rather than assuming.

---

## Staged Plan (as discussed)

1. Get the `bevy_ecs_ldtk` platformer example running as-is.
2. Wire up LDtk entity conventions (player spawn, goal, enemies/hazards) to match however the kids build levels in the editor.
3. Establish and verify determinism (fixed timestep, ordered systems, no incidental `HashMap`-order dependencies, seeded RNG).
4. Add IJON-style state annotations (e.g. `IJON_MAX(player_x)`) exposing Bevy ECS component state to a fuzzer.
5. (Optional, later) Evaluate whether a WASM build can serve as a portable, cross-machine-verifiable fuzzing/replay target.

---

## Source List

- IJON paper/repo: [github.com/RUB-SysSec/ijon](https://github.com/RUB-SysSec/ijon) (IEEE S&P 2020)
- AFL++: community-maintained fork incorporating IJON support
- Will Wilson (Antithesis), *"Testing a Single-Node, Single-Threaded, Distributed System Written in 1985"* (talk); *"Why Antithesis Works"* (blog post)
- Antithesis reference repos: `antithesishq/aardvark-arena`, `antithesishq/glitch-grid`
- LDtk: [ldtk.io](https://ldtk.io/), [github.com/deepnight/ldtk](https://github.com/deepnight/ldtk)
- `bevy_ecs_ldtk`: [github.com/Trouv/bevy_ecs_ldtk](https://github.com/Trouv/bevy_ecs_ldtk)
- WebAssembly numerics spec (determinism/NaN semantics): [webassembly.github.io/spec/core/exec/numerics.html](https://webassembly.github.io/spec/core/exec/numerics.html)
- WebAssembly design discussion on float determinism: [github.com/WebAssembly/design/issues/1385](https://github.com/WebAssembly/design/issues/1385)
- DFINITY forum, *"How is deterministic execution of WebAssembly ensured?"* (real-world deterministic-WASM runtime constraints): [forum.dfinity.org](https://forum.dfinity.org/t/how-is-deterministic-execution-of-webassembly-ensured/6335)
- WASM float determinism cross-platform/browser test harness: `wasm-float-determinism-test.pages.dev` ([github.com/meheleventyone/wasm-float-determinism-test](https://github.com/meheleventyone/wasm-float-determinism-test))
