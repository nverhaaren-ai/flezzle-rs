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
        app.add_plugins((LdtkPlugin, PhysicsPlugins::default()))
            .insert_resource(Gravity(Vec2::new(0.0, -2000.0)))
            .insert_resource(LevelSelection::Uid(0))
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                    load_level_neighbors: true,
                },
                set_clear_color: SetClearColor::FromLevelBackground,
                ..Default::default()
            })
            .add_plugins(game_flow::GameFlowPlugin)
            .add_plugins(walls::WallPlugin)
            .add_plugins(ground_detection::GroundDetectionPlugin)
            .add_plugins(climbing::ClimbingPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(enemy::EnemyPlugin)
            .add_systems(Update, inventory::dbg_print_inventory)
            .add_systems(Update, camera::camera_fit_inside_current_level)
            .add_plugins(misc_objects::MiscObjectsPlugin);
    }
}
