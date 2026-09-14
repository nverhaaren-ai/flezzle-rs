//! Windowed shell: DefaultPlugins + the game. Run with `--release`
//! (or rely on the dev-profile dependency optimization in Cargo.toml).

use bevy::prelude::*;
use flezzle_rs::GamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(GamePlugin)
        .run();
}
