//! Shared headless harness: the real game with no window, no GPU, no audio
//! or log output, and manually-advanced time — one 60 FPS frame (= one
//! simulation tick) per `app.update()`. Input is injected as `KeyboardInput`
//! messages, the same path a window backend uses.
#![allow(dead_code)]

use std::time::Duration;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;

use flezzle_rs::level::{register_user_level_source, LevelSource};
use flezzle_rs::player::Player;
use flezzle_rs::{GamePlugin, TICK_HZ};

/// One simulated frame; matches the game's tick so physics steps 1:1.
pub const FRAME: Duration = Duration::from_nanos((1e9 / TICK_HZ) as u64);

/// Headless app on the default level.
pub fn headless_app() -> App {
    headless_app_for(None)
}

/// Headless app on a specific level (an asset path such as
/// `levels/first-steps.ldtk`). The `user://` upload source is registered.
pub fn headless_app_for(level: Option<&str>) -> App {
    let mut app = App::new();
    register_user_level_source(&mut app);
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: None,
                    ..Default::default()
                })),
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            })
            .set(ImagePlugin::default_nearest()),
    );
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
    // Headless gap in bevy_render: the on_remove hook of render-synced
    // components (Sprite, Camera, ...) unwraps a resource that only
    // SyncWorldPlugin inserts, and RenderPlugin skips that plugin when there
    // is no render backend. Without it, despawning a level panics. The
    // plugin only touches the main world, so adding it here is harmless (its
    // queue is never drained, which is fine for short tests).
    app.add_plugins(bevy::render::sync_world::SyncWorldPlugin);
    if let Some(level) = level {
        app.insert_resource(LevelSource(level.to_owned()));
    }
    app.add_plugins(GamePlugin);

    // `app.run()` normally drives the plugin lifecycle to completion; we
    // drive updates manually, so finish it ourselves. (Some plugins insert
    // resources in `finish()` — e.g. Avian's spatial-query diagnostics.)
    for i in 0.. {
        match app.plugins_state() {
            bevy::app::PluginsState::Adding => std::thread::yield_now(),
            _ => break,
        }
        assert!(i < 100_000, "plugins never finished building");
    }
    app.finish();
    app.cleanup();
    app
}

/// Inject a key press/release as a `KeyboardInput` message. (Poking
/// `ButtonInput<KeyCode>` directly can't produce `just_pressed`, because the
/// input system clears the just-* sets every frame.)
pub fn send_key(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// Tick until `pred` is true, at most `max_frames` times. Returns success.
pub fn tick_until(app: &mut App, max_frames: usize, mut pred: impl FnMut(&mut App) -> bool) -> bool {
    for _ in 0..max_frames {
        app.update();
        if pred(app) {
            return true;
        }
    }
    false
}

pub fn player_pos(app: &mut App) -> Option<Vec2> {
    app.world_mut()
        .query_filtered::<&GlobalTransform, With<Player>>()
        .iter(app.world())
        .next()
        .map(|t| t.translation().truncate())
}

pub fn count<F: bevy::ecs::query::QueryFilter>(app: &mut App) -> usize {
    app.world_mut().query_filtered::<(), F>().iter(app.world()).count()
}

/// Tick until the player has spawned and physics has settled (the game
/// pauses the physics clock until the level finishes spawning). Generous
/// budget: most of these frames race async asset loading, not simulation.
pub fn spawn_and_settle(app: &mut App) -> Vec2 {
    let ready = tick_until(app, 3000, |app| player_pos(app).is_some());
    assert!(ready, "player never spawned");
    for _ in 0..120 {
        app.update();
    }
    player_pos(app).expect("player disappeared while settling")
}
