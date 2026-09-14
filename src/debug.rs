//! Observable game state for external test drivers.
//!
//! Once per rendered frame, [`DebugSnapshot`] is refreshed from the ECS. On
//! the web it is also published two ways: as `window.flezzle` (a live object,
//! handy in the devtools console) and as JSON text in a hidden
//! `<script type="application/json" id="flezzle-state">` element. The DOM
//! copy matters: tools that evaluate scripts in an *isolated world* (Bombadil,
//! browser extensions) share the DOM with the page but not its JS globals.
//! Natively, tests read the resource directly.
//!
//! Frame/tick accounting is here too: a rendered frame may run 0, 1, or 2+
//! simulation ticks, and the counters make that visible — which is what turns
//! "the world visibly jumps sometimes" into a checkable property.

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use serde::Serialize;

use crate::climbing::Climber;
use crate::enemy::Enemy;
use crate::ground_detection::GroundDetection;
use crate::input::TickInput;
use crate::level::LevelSource;
use crate::misc_objects::Chest;
use crate::player::Player;

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2Snap {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for Vec2Snap {
    fn from(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct RectSnap {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BodySnap {
    pub pos: Vec2Snap,
    /// Draw depth (Bevy `z`); larger is drawn on top.
    pub z: f32,
    pub vel: Vec2Snap,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PlayerSnap {
    #[serde(flatten)]
    pub body: BodySnap,
    pub on_ground: bool,
    pub climbing: bool,
}

#[derive(Resource, Serialize, Clone, Debug, Default)]
pub struct DebugSnapshot {
    /// Rendered frames so far.
    pub frame: u64,
    /// Simulation ticks so far (see `TickInput`).
    pub tick: u64,
    /// Simulation ticks that ran during the frame being reported.
    pub ticks_this_frame: u32,
    /// Frames in which no tick ran / more than one tick ran, since start.
    pub zero_tick_frames: u64,
    pub multi_tick_frames: u64,
    /// Actions held during the current tick (bit layout: `input::Action`).
    pub held_actions: u8,
    /// Which level file is loaded and which level within it is selected.
    pub level: String,
    pub level_selection: String,
    /// LDtk level lifecycle counters (LevelEvent messages seen).
    pub level_spawns: u64,
    pub level_despawns: u64,
    pub physics_paused: bool,
    pub player: Option<PlayerSnap>,
    /// The first chest, if any.
    pub chest: Option<BodySnap>,
    pub mobs: Vec<BodySnap>,
    /// Camera bottom-left corner in world space (the game's projection uses
    /// viewport origin (0, 0)).
    pub camera: Option<Vec2Snap>,
    /// World-space rectangle of the selected level (bottom-left + size), once
    /// known. Lets a checker ask "is the player still inside the level?".
    pub level_bounds: Option<RectSnap>,
    /// Largest *horizontal* player displacement between two consecutive
    /// rendered frames in which the player was on the ground, over the last
    /// [`STEP_WINDOW`] frames, in pixels. Jumps and falls are excluded.
    pub max_walk_step: f32,
    /// Worst relative error of the *rendered* walking speed over the last
    /// [`STEP_WINDOW`] frames: `|dx / frame_seconds - WALK_SPEED| / WALK_SPEED`,
    /// measured only across frames where the player was grounded and holding
    /// left or right in both. Smooth rendering keeps this near 0 at any frame
    /// rate; a 60 Hz tick rendered at a drifting refresh rate without
    /// interpolation alternates frames of 0 and 2 ticks, so it approaches 1.
    /// This is the "world visibly jumps" symptom as a number.
    pub max_walk_speed_error: f32,
}

/// Player walking speed in px/s (see `player_movement`).
pub const WALK_SPEED: f32 = 200.0;

/// Frames over which `max_player_step` is tracked.
pub const STEP_WINDOW: usize = 120;

#[derive(Resource, Default)]
struct StepHistory {
    /// Position, grounded flag, and "walking input held" flag from the
    /// previous frame.
    last: Option<(Vec2, bool, bool)>,
    steps: std::collections::VecDeque<f32>,
    speed_errors: std::collections::VecDeque<f32>,
}

/// Counts fixed ticks between snapshot refreshes.
#[derive(Resource, Default)]
struct TickCounter(u32);

fn count_tick(mut counter: ResMut<TickCounter>) {
    counter.0 += 1;
}

#[allow(clippy::too_many_arguments)]
fn refresh_snapshot(
    mut snapshot: ResMut<DebugSnapshot>,
    mut counter: ResMut<TickCounter>,
    mut history: ResMut<StepHistory>,
    real_time: Res<Time<Real>>,
    tick_input: Res<TickInput>,
    level_source: Res<LevelSource>,
    level_selection: Res<LevelSelection>,
    physics_time: Res<Time<Physics>>,
    mut level_events: MessageReader<LevelEvent>,
    player: Query<(&GlobalTransform, &LinearVelocity, &GroundDetection, &Climber), With<Player>>,
    chests: Query<(&GlobalTransform, &LinearVelocity), With<Chest>>,
    mobs: Query<(&GlobalTransform, &LinearVelocity), With<Enemy>>,
    camera: Query<&GlobalTransform, With<Camera>>,
    levels: Query<(&GlobalTransform, &LevelIid)>,
    projects: Query<&LdtkProjectHandle>,
    project_assets: Res<Assets<LdtkProject>>,
) {
    let ticks = std::mem::take(&mut counter.0);
    snapshot.frame += 1;
    snapshot.ticks_this_frame = ticks;
    match ticks {
        0 => snapshot.zero_tick_frames += 1,
        1 => {}
        _ => snapshot.multi_tick_frames += 1,
    }
    snapshot.tick = tick_input.tick();
    snapshot.held_actions = tick_input.held().bits();
    snapshot.level = level_source.0.clone();
    snapshot.level_selection = format!("{:?}", *level_selection);
    snapshot.physics_paused = physics_time.is_paused();
    for event in level_events.read() {
        match event {
            LevelEvent::Spawned(_) => snapshot.level_spawns += 1,
            LevelEvent::Despawned(_) => snapshot.level_despawns += 1,
            _ => {}
        }
    }

    let body = |t: &GlobalTransform, v: &LinearVelocity| BodySnap {
        pos: t.translation().truncate().into(),
        z: t.translation().z,
        vel: v.0.into(),
    };
    snapshot.player = player.iter().next().map(|(t, v, ground, climber)| PlayerSnap {
        body: body(t, v),
        on_ground: ground.on_ground,
        climbing: climber.climbing,
    });
    let walking_input = {
        use crate::input::Action;
        tick_input.pressed(Action::Left) != tick_input.pressed(Action::Right)
    };
    let player_now = snapshot
        .player
        .as_ref()
        .map(|p| (Vec2::new(p.body.pos.x, p.body.pos.y), p.on_ground, walking_input));
    if let (Some((prev, prev_grounded, prev_walking)), Some((now, grounded, walking))) =
        (history.last, player_now)
    {
        // Only grounded-to-grounded frames count as "walking".
        let dx = if prev_grounded && grounded { (now.x - prev.x).abs() } else { 0.0 };
        history.steps.push_back(dx);
        // Speed error only while walking input was held across both frames
        // and the frame had a measurable duration.
        let dt = real_time.delta_secs();
        let err = if prev_grounded && grounded && prev_walking && walking && dt > 1e-4 {
            ((dx / dt) - WALK_SPEED).abs() / WALK_SPEED
        } else {
            0.0
        };
        history.speed_errors.push_back(err);
        if history.steps.len() > STEP_WINDOW {
            history.steps.pop_front();
            history.speed_errors.pop_front();
        }
    }
    history.last = player_now;
    snapshot.max_walk_step = history.steps.iter().copied().fold(0.0, f32::max);
    snapshot.max_walk_speed_error = history.speed_errors.iter().copied().fold(0.0, f32::max);
    snapshot.chest = chests.iter().next().map(|(t, v)| body(t, v));
    snapshot.mobs = mobs.iter().map(|(t, v)| body(t, v)).collect();
    snapshot.camera = camera.iter().next().map(|t| t.translation().truncate().into());
    snapshot.level_bounds = projects
        .iter()
        .next()
        .and_then(|h| project_assets.get(h))
        .and_then(|project| {
            levels.iter().find_map(|(transform, iid)| {
                let level = project.get_raw_level_by_iid(&iid.to_string())?;
                level_selection
                    .is_match(&LevelIndices::default(), level)
                    .then(|| RectSnap {
                        x: transform.translation().x,
                        y: transform.translation().y,
                        w: level.px_wid as f32,
                        h: level.px_hei as f32,
                    })
            })
        });

    #[cfg(target_arch = "wasm32")]
    web::publish(&snapshot);
}

pub struct DebugSnapshotPlugin;

impl Plugin for DebugSnapshotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSnapshot>()
            .init_resource::<TickCounter>()
            .init_resource::<StepHistory>()
            .add_systems(FixedFirst, count_tick)
            // `Last`: after the fixed loop, transform propagation, and camera
            // fitting for this frame.
            .add_systems(Last, refresh_snapshot);
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::DebugSnapshot;
    use wasm_bindgen::JsValue;

    pub const STATE_ELEMENT_ID: &str = "flezzle-state";

    /// Publish the snapshot as `window.flezzle` and as DOM text.
    pub fn publish(snapshot: &DebugSnapshot) {
        let Some(window) = web_sys::window() else { return };
        let Ok(json) = serde_json::to_string(snapshot) else { return };
        if let Ok(value) = js_sys::JSON::parse(&json) {
            let _ = js_sys::Reflect::set(&window, &JsValue::from_str("flezzle"), &value);
        }
        let Some(document) = window.document() else { return };
        let element = match document.get_element_by_id(STATE_ELEMENT_ID) {
            Some(e) => e,
            None => {
                let Ok(e) = document.create_element("script") else { return };
                let _ = e.set_attribute("type", "application/json");
                e.set_id(STATE_ELEMENT_ID);
                if let Some(body) = document.body() {
                    let _ = body.append_child(&e);
                }
                e
            }
        };
        element.set_text_content(Some(&json));
    }
}
