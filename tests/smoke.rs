//! Headless smoke tests for the platformer. Green is a floor, not a finish
//! line: these don't cover climbing, restart, or every movement rule — the
//! workbook's manual play checklist is part of the contract too.
//!
//! In the milestone-01 start state these tests are RED. Implementing the
//! workbook project's tasks turns them green. Don't edit this file to make
//! them pass.

mod common;

use avian2d::prelude::Gravity;
use bevy::input::keyboard::Key;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use common::*;
use flezzle_rs::{player::Player, walls::Wall};

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
        count::<With<Player>>(app) == 1 && count::<With<Wall>>(app) > 0
    });

    assert!(
        spawned,
        "expected exactly one Player and some Wall entities within 3000 frames \
         of loading the default level"
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
    send_key(&mut app, KeyCode::KeyD, Key::Character("d".into()), ButtonState::Pressed);
    for _ in 0..60 {
        app.update();
    }
    send_key(&mut app, KeyCode::KeyD, Key::Character("d".into()), ButtonState::Released);

    let after = player_pos(&mut app).expect("player disappeared");
    assert!(
        after.x > before.x + 10.0,
        "holding D for 60 frames should move the player right \
         (before: {}, after: {})",
        before.x,
        after.x
    );
}
