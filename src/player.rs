use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

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
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut LinearVelocity, &mut Climber, &GroundDetection), With<Player>>,
) {
    // TODO(project-01): implement the player controls.
    //
    // Desired behavior, for each player entity in `query`:
    //   - A/D set horizontal velocity: 200 px/s left/right (0 if neither or
    //     both are held).
    //   - Climbing: if the climber intersects no climbables, it is not
    //     climbing; pressing W or S while touching a climbable starts
    //     climbing. While climbing, W/S set vertical velocity to ±200 px/s
    //     (0 if neither or both).
    //   - Space jumps (vertical velocity 500) when on the ground or climbing,
    //     and cancels climbing.
    //
    // Velocity is Avian's `LinearVelocity` (`.x` / `.y`). Key state comes
    // from `ButtonInput<KeyCode>` (`pressed` vs `just_pressed` — think about
    // which fits where).
    let _ = (&input, &mut query);
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, player_movement)
            .register_ldtk_entity::<PlayerBundle>("Player");
    }
}
