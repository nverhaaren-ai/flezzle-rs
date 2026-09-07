//! Which level to play, switching levels at runtime, and loading levels
//! from bytes (the browser's file picker) through an in-memory asset source.

use std::path::Path;

use avian2d::prelude::*;
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

/// Level loaded when nothing else is asked for.
pub const DEFAULT_LEVEL: &str = "levels/example_world.ldtk";

/// Name of the in-memory asset source that holds uploaded levels.
pub const USER_SOURCE: &str = "user";

/// Asset path of the LDtk project to play (e.g. `levels/first-steps.ldtk`
/// or `user://levels/upload-1.ldtk`). Changing it reloads the world.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct LevelSource(pub String);

impl Default for LevelSource {
    fn default() -> Self {
        Self(DEFAULT_LEVEL.to_owned())
    }
}

/// Marker on the spawned LDtk world root so it can be despawned on reload.
#[derive(Component)]
pub struct LevelWorld;

/// In-memory directory behind the `user://` asset source. Shells that want
/// upload support call [`register_user_level_source`] before `AssetPlugin`.
#[derive(Resource, Clone)]
pub struct UserLevelDir(pub Dir);

/// Register the `user://` in-memory asset source, pre-populated with the
/// bundled tilesets so an uploaded level's relative tileset paths resolve.
/// Must run before `DefaultPlugins` / `AssetPlugin` are added.
pub fn register_user_level_source(app: &mut App) -> UserLevelDir {
    let dir = Dir::default();
    dir.insert_asset(
        Path::new("atlas/SunnyLand_by_Ansimuz-extended.png"),
        include_bytes!("../assets/atlas/SunnyLand_by_Ansimuz-extended.png").to_vec(),
    );
    dir.insert_asset(
        Path::new("atlas/MV Icons Complete Sheet Free - ALL.png"),
        include_bytes!("../assets/atlas/MV Icons Complete Sheet Free - ALL.png").to_vec(),
    );
    let reader_root = dir.clone();
    app.register_asset_source(
        AssetSourceId::from(USER_SOURCE),
        AssetSourceBuilder::new(move || {
            Box::new(MemoryAssetReader {
                root: reader_root.clone(),
            })
        }),
    );
    let user_dir = UserLevelDir(dir);
    app.insert_resource(user_dir.clone());
    user_dir
}

/// Make `bytes` (an `.ldtk` file) the current level. Each call uses a fresh
/// path so the asset server doesn't hand back a previously loaded project.
pub fn load_level_bytes(
    dir: &UserLevelDir,
    source: &mut LevelSource,
    upload_counter: &mut u32,
    bytes: Vec<u8>,
) {
    *upload_counter += 1;
    let path = format!("levels/upload-{upload_counter}.ldtk");
    dir.0.insert_asset(Path::new(&path), bytes);
    source.0 = format!("{USER_SOURCE}://{path}");
}

/// (Re)spawn the LDtk world whenever [`LevelSource`] changes — including
/// its first appearance. Physics is paused until the level has spawned
/// (see `game_flow::start_physics`).
fn spawn_level_on_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    source: Res<LevelSource>,
    existing: Query<Entity, With<LevelWorld>>,
    mut physics_time: ResMut<Time<Physics>>,
    mut level_selection: ResMut<LevelSelection>,
) {
    if !source.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    physics_time.pause();
    *level_selection = LevelSelection::index(0);
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load(source.0.clone()).into(),
            ..Default::default()
        },
        LevelWorld,
    ));
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelSource>()
            .insert_resource(LevelSelection::index(0))
            .add_systems(
                FixedUpdate,
                spawn_level_on_change.in_set(crate::GameplaySet::World),
            );
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, web::poll_uploaded_level);
    }
}

/// Browser glue: level choice from the page URL, uploads from the page's
/// file picker (which stashes bytes on `window.flezzlePendingLevel`).
#[cfg(target_arch = "wasm32")]
pub mod web {
    use super::*;
    use js_sys::{Reflect, Uint8Array};
    use wasm_bindgen::{JsCast, JsValue};

    const PENDING_KEY: &str = "flezzlePendingLevel";

    /// `?level=levels/foo.ldtk` from the page URL, if present.
    pub fn level_from_query() -> Option<String> {
        let search = web_sys::window()?.location().search().ok()?;
        let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
        params.get("level").filter(|s| !s.is_empty())
    }

    fn take_pending_upload() -> Option<Vec<u8>> {
        let window = web_sys::window()?;
        let key = JsValue::from_str(PENDING_KEY);
        let value = Reflect::get(&window, &key).ok()?;
        if value.is_undefined() || value.is_null() {
            return None;
        }
        Reflect::set(&window, &key, &JsValue::UNDEFINED).ok()?;
        Some(value.dyn_into::<Uint8Array>().ok()?.to_vec())
    }

    pub fn poll_uploaded_level(
        dir: Option<Res<UserLevelDir>>,
        mut source: ResMut<LevelSource>,
        mut upload_counter: Local<u32>,
    ) {
        let Some(dir) = dir else { return };
        if let Some(bytes) = take_pending_upload() {
            load_level_bytes(&dir, &mut source, &mut upload_counter, bytes);
        }
    }
}
