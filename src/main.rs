//! Windowed shell: DefaultPlugins + the game, natively or in a browser.
//!
//! Level choice: first CLI argument natively (`cargo run -- levels/foo.ldtk`),
//! `?level=levels/foo.ldtk` on the web. The web page can also hand us an
//! uploaded `.ldtk` file (see `web/index.html`).

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use flezzle_rs::level::{register_user_level_source, LevelSource};
use flezzle_rs::GamePlugin;

fn main() {
    let mut app = App::new();

    // Must precede DefaultPlugins (AssetPlugin) so the `user://` source exists.
    register_user_level_source(&mut app);

    app.add_plugins(
        DefaultPlugins
            .build()
            // No sounds yet; on the web an AudioContext also can't start
            // before a user gesture, which only produces console warnings.
            .disable::<bevy::audio::AudioPlugin>()
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                // Don't probe for `.meta` sidecar files: on the web every
                // asset load would first 404 on `<asset>.meta`.
                meta_check: AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "flezzle-rs".into(),
                    #[cfg(target_arch = "wasm32")]
                    canvas: Some("#flezzle-canvas".into()),
                    #[cfg(target_arch = "wasm32")]
                    fit_canvas_to_parent: true,
                    ..Default::default()
                }),
                ..Default::default()
            }),
    );

    if let Some(level) = requested_level() {
        app.insert_resource(LevelSource(level));
    }

    app.add_plugins(GamePlugin).run();
}

#[cfg(not(target_arch = "wasm32"))]
fn requested_level() -> Option<String> {
    std::env::args()
        .nth(1)
        .or_else(|| std::env::var("FLEZZLE_LEVEL").ok())
}

#[cfg(target_arch = "wasm32")]
fn requested_level() -> Option<String> {
    flezzle_rs::level::web::level_from_query()
}
