//! Headless smoke tests for the platformer.
//!
//! These run the real game (via [`flezzle_rs::GamePlugin`]) with no window,
//! no GPU, and manually-advanced time, injecting keyboard input as data
//! (`KeyboardInput` messages) — an early sketch of the replay architecture
//! this project is building toward. "Sketch" deliberately: green tests are a
//! floor, not a finish line. They don't cover climbing, restart, or every
//! movement rule — the workbook's manual play checklist is part of the
//! contract too.
//!
//! In the milestone-01 start state these tests are RED. Implementing the
//! workbook project's tasks turns them green. Don't edit this file to make
//! them pass.

use std::time::Duration;

use avian2d::prelude::Gravity;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy_ecs_ldtk::prelude::*;

use flezzle_rs::{player::Player, walls::Wall, GamePlugin};

/// One simulated frame at 60 FPS.
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// Build the game app with no window, no GPU backend, no audio/log output,
/// and manually-advanced time: exactly one 60 FPS frame per `app.update()`.
///
/// Bevy's fixed-timestep clock (which Avian's physics runs on) defaults to
/// 64 Hz; we pin it to 60 Hz so one `update()` advances exactly one physics
/// step. Reconciling the render/update/physics clocks properly is milestone
/// 02's problem; this keeps the tests exact meanwhile.
fn headless_app() -> App {
    let mut app = App::new();
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
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
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

/// Inject a key press/release the same way a real window backend would:
/// as a `KeyboardInput` message consumed by Bevy's input systems. (Poking
/// `ButtonInput<KeyCode>` directly can't produce `just_pressed`, because
/// the input system clears the just-* sets every frame.)
fn send_key(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState) {
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
fn tick_until(app: &mut App, max_frames: usize, mut pred: impl FnMut(&mut App) -> bool) -> bool {
    for _ in 0..max_frames {
        app.update();
        if pred(app) {
            return true;
        }
    }
    false
}

fn player_pos(app: &mut App) -> Option<Vec2> {
    app.world_mut()
        .query_filtered::<&GlobalTransform, With<Player>>()
        .iter(app.world())
        .next()
        .map(|t| t.translation().truncate())
}

/// Tick until the player has spawned and physics has settled (the game
/// pauses the physics clock until the level finishes spawning).
fn spawn_and_settle(app: &mut App) -> Vec2 {
    // Generous budget: most of these frames race async asset loading, not
    // simulation work, so headless frames are cheap.
    let ready = tick_until(app, 3000, |app| player_pos(app).is_some());
    assert!(ready, "player never spawned");
    for _ in 0..120 {
        app.update();
    }
    player_pos(app).expect("player disappeared while settling")
}

#[test]
fn game_plugin_inserts_core_resources() {
    let app = headless_app();

    assert!(
        app.world().contains_resource::<LevelSelection>(),
        "GamePlugin should insert a LevelSelection so LDtk knows which level to spawn"
    );

    // Note: LdtkPlugin itself init_resources a default LdtkSettings, so we
    // assert the configured values, not mere existence.
    let settings = app
        .world()
        .get_resource::<LdtkSettings>()
        .expect("LdtkSettings should exist once LdtkPlugin is added");
    assert_eq!(
        settings.level_spawn_behavior,
        LevelSpawnBehavior::UseWorldTranslation {
            load_level_neighbors: true
        },
        "levels should spawn at world translation, loading neighbors"
    );
    assert_eq!(
        settings.set_clear_color,
        SetClearColor::FromLevelBackground,
        "clear color should come from the level background"
    );

    let gravity = app
        .world()
        .get_resource::<Gravity>()
        .expect("Avian's PhysicsPlugins should be added (Gravity resource missing)");
    assert_eq!(
        gravity.0,
        Vec2::new(0.0, -2000.0),
        "gravity should be configured for this level's tuning"
    );
}

#[test]
fn level_spawns_player_and_walls() {
    let mut app = headless_app();

    let spawned = tick_until(&mut app, 3000, |app| {
        let players = app
            .world_mut()
            .query_filtered::<(), With<Player>>()
            .iter(app.world())
            .count();
        let walls = app
            .world_mut()
            .query_filtered::<(), With<Wall>>()
            .iter(app.world())
            .count();
        players == 1 && walls > 0
    });

    assert!(
        spawned,
        "expected exactly one Player and some Wall entities within 3000 frames \
         of loading Typical_2D_platformer_example.ldtk"
    );
}

#[test]
fn player_jumps_and_moves_right() {
    let mut app = headless_app();
    let start = spawn_and_settle(&mut app);

    // --- Jump: press Space, expect upward motion, then a return to ground.
    let mut peak = start.y;
    send_key(&mut app, KeyCode::Space, Key::Space, ButtonState::Pressed);
    for _ in 0..30 {
        app.update();
        if let Some(p) = player_pos(&mut app) {
            peak = peak.max(p.y);
        }
    }
    assert!(
        peak > start.y + 5.0,
        "pressing Space while grounded should launch the player upward \
         (start y: {}, peak y: {peak})",
        start.y
    );
    send_key(&mut app, KeyCode::Space, Key::Space, ButtonState::Released);

    // Land and settle again before testing horizontal movement.
    for _ in 0..120 {
        app.update();
    }
    let before = player_pos(&mut app).expect("player disappeared");

    // --- Move right: hold D for one simulated second.
    send_key(
        &mut app,
        KeyCode::KeyD,
        Key::Character("d".into()),
        ButtonState::Pressed,
    );
    for _ in 0..60 {
        app.update();
    }
    send_key(
        &mut app,
        KeyCode::KeyD,
        Key::Character("d".into()),
        ButtonState::Released,
    );

    let after = player_pos(&mut app).expect("player disappeared");
    assert!(
        after.x > before.x + 10.0,
        "holding D for 60 frames should move the player right \
         (before: {}, after: {})",
        before.x,
        after.x
    );
}
