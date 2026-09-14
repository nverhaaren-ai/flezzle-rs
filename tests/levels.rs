//! Every bundled level loads and is playable-in-principle: a player spawns,
//! walls exist. Also exercises the `user://` upload path headlessly.

mod common;

use std::path::Path;

use bevy::prelude::*;

use common::*;
use flezzle_rs::level::{load_level_bytes, LevelSource, UserLevelDir};
use flezzle_rs::{player::Player, walls::Wall};

fn bundled_levels() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .expect("assets/levels should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ldtk"))
        .map(|p| format!("levels/{}", p.file_name().unwrap().to_string_lossy()))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no bundled levels found");
    out
}

fn assert_playable(app: &mut App, what: &str) {
    let ok = tick_until(app, 3000, |app| {
        count::<With<Player>>(app) == 1 && count::<With<Wall>>(app) > 0
    });
    assert!(ok, "{what}: expected one Player and some walls within 3000 frames");
    // A little physics: the player should still exist and be inside the level.
    let pos = spawn_and_settle(app);
    assert!(pos.y > -1000.0, "{what}: player fell out of the world (y = {})", pos.y);
}

#[test]
fn every_bundled_level_is_playable() {
    for level in bundled_levels() {
        let mut app = headless_app_for(Some(&level));
        assert_playable(&mut app, &level);
    }
}

#[test]
fn levels_manifest_lists_existing_files() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels/index.json");
    let text = std::fs::read_to_string(manifest).expect("assets/levels/index.json");
    let bundled = bundled_levels();
    // Cheap check without a JSON dependency: every "path" value must be bundled.
    let mut seen = 0;
    for line in text.lines().filter(|l| l.contains("\"path\"")) {
        let after_key = line.split("\"path\"").nth(1).expect("path key");
        let path = after_key.split('"').nth(1).expect("path value");
        assert!(bundled.contains(&path.to_owned()), "manifest lists missing level {path}");
        seen += 1;
    }
    assert!(seen > 0, "manifest lists no levels");
}

#[test]
fn switching_levels_reloads_the_world() {
    let mut app = headless_app_for(Some("levels/template.ldtk"));
    assert_playable(&mut app, "template");
    let template_walls = count::<With<Wall>>(&mut app);

    app.world_mut().resource_mut::<LevelSource>().0 = "levels/ladder-tower.ldtk".into();
    let switched = tick_until(&mut app, 3000, |app| {
        count::<With<Player>>(app) == 1 && count::<With<Wall>>(app) != template_walls
    });
    assert!(switched, "changing LevelSource should replace the world");
}

#[test]
fn uploaded_level_bytes_load_through_user_source() {
    let mut app = headless_app_for(Some("levels/template.ldtk"));
    assert_playable(&mut app, "template");

    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels/first-steps.ldtk"),
    )
    .unwrap();
    let dir = app.world().resource::<UserLevelDir>().clone();
    let mut counter = 0;
    load_level_bytes(
        &dir,
        &mut app.world_mut().resource_mut::<LevelSource>(),
        &mut counter,
        bytes,
    );
    assert!(
        app.world().resource::<LevelSource>().0.starts_with("user://"),
        "upload should switch to the user:// source"
    );
    // first-steps has two mobs; template has none — distinguishes the levels.
    let loaded = tick_until(&mut app, 3000, |app| {
        count::<With<flezzle_rs::enemy::Enemy>>(app) == 2
    });
    assert!(loaded, "uploaded level should spawn through the in-memory source");
}

/// The camera must frame the level the player is in. The upstream camera code
/// matches levels against `LevelSelection`; with an index-based selection the
/// placeholder indices it passes match *every* level, so the camera framed
/// whichever level came last. Regression test for the fix (pinning the
/// selection to the spawned level's iid).
#[test]
fn camera_frames_the_players_level() {
    use bevy::camera::Camera;
    let mut app = headless_app_for(Some("levels/example_world.ldtk"));
    let player = spawn_and_settle(&mut app);

    let cam = app
        .world_mut()
        .query_filtered::<&GlobalTransform, With<Camera>>()
        .iter(app.world())
        .next()
        .map(|t| t.translation().truncate())
        .expect("camera");

    // Main level of example_world is 848x336 at LDtk world (0, 0). LDtk is
    // y-down, so with UseWorldTranslation its Bevy origin is (0, -336). The
    // camera's translation is its bottom-left corner (viewport_origin = 0);
    // it must lie inside that level and the player must be within the framed
    // 16:9 window.
    let (level_x, level_y) = (0.0, -336.0);
    assert!(
        (level_x..=level_x + 848.0).contains(&cam.x) && (level_y..=level_y + 336.0).contains(&cam.y),
        "camera bottom-left {cam} is outside the player's level"
    );
    let framed_w = 336.0 * 16.0 / 9.0;
    assert!(
        (cam.x..=cam.x + framed_w).contains(&player.x) && (cam.y..=cam.y + 336.0).contains(&player.y),
        "player {player} is not inside the camera window starting at {cam}"
    );
}
