# Attribution

## Code

The gameplay code for the initial platformer milestone is adapted from the
[`bevy_ecs_ldtk` platformer example](https://github.com/Trouv/bevy_ecs_ldtk/tree/v0.15.0/examples/platformer)
(v0.15.0) by Trevor Lovell and contributors, licensed under
[MIT](https://github.com/Trouv/bevy_ecs_ldtk/blob/v0.15.0/LICENSE) OR Apache-2.0.
Modifications: restructured as a library crate (`GamePlugin` in `src/lib.rs`)
with a thin windowed binary, plus headless smoke tests; fixed a zero-length
`normalize()` NaN in the enemy patrol system (surfaced by fixed-tick
simulation).

This crate is licensed Apache-2.0 — a permitted election from the upstream
example's MIT OR Apache-2.0 dual license.

## Assets

- `assets/Typical_2D_platformer_example.ldtk` — sample level distributed with
  the [`bevy_ecs_ldtk`](https://github.com/Trouv/bevy_ecs_ldtk) repository
  (MIT OR Apache-2.0), authored in [LDtk](https://ldtk.io/); it references
  the two atlas images below as tilesets.
- `assets/player.png`, `assets/atlas/SunnyLand_by_Ansimuz-extended.png` —
  from [SunnyLand](https://ansimuz.itch.io/sunny-land-pixel-game-art), a
  texture pack by Ansimuz, licensed under
  [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
- `assets/atlas/MV Icons Complete Sheet Free - ALL.png` — from
  [PIXEL FANTASY RPG ICONS](https://cazwolf.itch.io/caz-pixel-free), an icon
  pack by Caz, licensed under
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
