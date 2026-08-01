//! flezzle-rs — deterministic platformer level player (early bootstrap).
//!
//! Gameplay code in this milestone is adapted from the `bevy_ecs_ldtk`
//! platformer example (v0.15.0), MIT OR Apache-2.0 — see ATTRIBUTION.md.
//!
//! The crate is a library so that the game can be assembled in different
//! shells: the windowed binary (`src/main.rs`), headless tests
//! (`tests/smoke.rs`), and — later — WASM and fuzzing harnesses.
//! [`GamePlugin`] owns all gameplay wiring; shells own platform plugins
//! (window/render/input backends).

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

pub mod camera;
pub mod climbing;
pub mod colliders;
pub mod enemy;
pub mod game_flow;
pub mod ground_detection;
pub mod inventory;
pub mod misc_objects;
pub mod player;
pub mod walls;

/// All gameplay wiring: LDtk loading, physics, and the game's systems.
///
/// Deliberately excludes `DefaultPlugins` — the shell decides how (and
/// whether) to render.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // TODO(project-01): assemble the game. Broadly, this plugin needs to:
        //
        //   1. Add the third-party plugins: `LdtkPlugin` (loads .ldtk files as
        //      Bevy assets and spawns levels) and Avian's `PhysicsPlugins`.
        //   2. Configure the world: a `Gravity` resource (the reference level
        //      plays well with a strong downward pull, e.g. (0, -2000)), a
        //      `LevelSelection` (the level with Uid 0), and `LdtkSettings`
        //      (spawn levels at their world translation, loading neighbors;
        //      clear color from the level background).
        //   3. Add this crate's gameplay plugins/systems (see the modules
        //      above): game flow, walls, ground detection, climbing, player,
        //      enemies, misc objects — plus the `inventory::dbg_print_inventory`
        //      and `camera::camera_fit_inside_current_level` systems.
        //
        // The upstream `bevy_ecs_ldtk` platformer example is the reference —
        // see the workbook project README for links and study notes.
        //
        // (The unused-import warnings on this file disappear as you wire
        // things up — they're a hint about which preludes you'll need.)
        let _ = app;
    }
}
