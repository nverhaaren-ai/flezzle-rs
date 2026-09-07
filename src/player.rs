use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::input::{Action, TickInput};
use crate::{climbing::Climber, inventory::Inventory};
use crate::{colliders::ColliderBundle, ground_detection::GroundDetection};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component)]
pub struct Player;

#[derive(Clone, Default, Bundle, LdtkEntity)]
pub struct PlayerBundle {
    #[sprite("player.png")]
    pub sprite: Sprite,
    #[from_entity_instance]
    pub collider_bundle: ColliderBundle,
    pub player: Player,
    #[worldly]
    pub worldly: Worldly,
    pub climber: Climber,
    pub ground_detection: GroundDetection,

    // Build Items Component manually by using `impl From<&EntityInstance>`
    #[from_entity_instance]
    items: Inventory,

    // The whole EntityInstance can be stored directly as an EntityInstance component
    #[from_entity_instance]
    entity_instance: EntityInstance,
}

pub fn player_movement(
    input: Res<TickInput>,
    mut query: Query<(&mut LinearVelocity, &mut Climber, &GroundDetection), With<Player>>,
) {
    for (mut velocity, mut climber, ground_detection) in &mut query {
        let right = if input.pressed(Action::Right) { 1. } else { 0. };
        let left = if input.pressed(Action::Left) { 1. } else { 0. };

        velocity.x = (right - left) * 200.;

        if climber.intersecting_climbables.is_empty() {
            climber.climbing = false;
        } else if input.just_pressed(Action::Up) || input.just_pressed(Action::Down) {
            climber.climbing = true;
        }

        if climber.climbing {
            let up = if input.pressed(Action::Up) { 1. } else { 0. };
            let down = if input.pressed(Action::Down) { 1. } else { 0. };

            velocity.y = (up - down) * 200.;
        }

        if input.just_pressed(Action::Jump) && (ground_detection.on_ground || climber.climbing) {
            velocity.y = 500.;
            climber.climbing = false;
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, player_movement.in_set(crate::GameplaySet::Act))
            .register_ldtk_entity::<PlayerBundle>("Player");
    }
}
