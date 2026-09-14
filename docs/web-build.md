# Web build

The browser version of flezzle-rs is a **static bundle**: `dist/index.html`
plus the compiled `.wasm`, its JS glue, and a copy of `assets/`. Any static
file server can host it (GitHub Pages, `python3 -m http.server`, itch.io).
It cannot be opened from `file://` — browsers block the `fetch` calls the
wasm loader and asset loading need — so "run it locally" means "serve the
folder".

## One-time setup

```bash
rustup target add wasm32-unknown-unknown
# Trunk drives wasm-bindgen / wasm-opt with versions matching Cargo.lock.
# Prebuilt binary (fast):
curl -sSL https://github.com/trunk-rs/trunk/releases/latest/download/trunk-x86_64-unknown-linux-gnu.tar.gz \
  | tar xz -C ~/.cargo/bin trunk
# or: cargo install trunk --locked
```

## Build and run

```bash
trunk build --cargo-profile wasm-release     # -> dist/   (first build: many minutes)
python3 -m http.server -d dist 8080          # then open http://localhost:8080/
```

Dev loop with rebuild-on-save: `trunk serve --cargo-profile wasm-release`
(serves on <http://127.0.0.1:8080>). Drop `--cargo-profile wasm-release`
for a much faster, much larger debug bundle.

Pick a level from the page's drop-down (backed by `assets/levels/index.json`),
via `?level=levels/first-steps.ldtk` in the URL, or open any `.ldtk` file
from disk with the file picker — it is handed to the game as bytes and
never leaves your machine. Click the canvas if keys don't register.

## What's configured where

| Piece | File | Notes |
|---|---|---|
| Page, level picker, upload glue | `web/index.html` | The game polls `window.flezzlePendingLevel` for uploads |
| Trunk settings | `Trunk.toml` | `public_url = "./"` keeps the bundle relocatable (subdirectory hosting) |
| No SRI hashes | `web/index.html` `data-integrity="none"` | Test drivers that instrument JS for coverage (Bombadil) rewrite the module files; with integrity hashes the game silently never boots |
| Size-optimised profile | `Cargo.toml` `[profile.wasm-release]` | `opt-level = "s"`, fat LTO; Trunk also runs `wasm-opt -Os` |
| wasm-only deps | `Cargo.toml` `[target.'cfg(target_arch = "wasm32")']` | `web-sys`/`js-sys` for the URL and upload glue; `getrandom` JS backend |
| getrandom backend cfg | `.cargo/config.toml` | Required by `getrandom` 0.3 on `wasm32-unknown-unknown` |
| Renderer | `bevy` feature `webgl2` | WebGL2 for compatibility; WebGPU later |
| Tilemaps | `bevy_ecs_ldtk` feature `atlas` | WebGL2 has no texture arrays |

Threads and SIMD are off (Bevy's wasm default) — deliberately, per the
determinism notes in `ldtk-fuzzing-research-summary.md`.

## Known gaps

- No audio (nothing plays sound yet; browsers also require a user gesture
  before audio can start).
- The tick rate is 60 Hz regardless of display refresh; rendering runs at
  the display's rate. Replay determinism across native and web is milestone
  02's job (F4) — not yet verified.
- Bundle size is a few MB compressed; first load on a slow connection is
  noticeable.
