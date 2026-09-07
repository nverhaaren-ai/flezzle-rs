//! Tick-driven input.
//!
//! Gameplay systems run on the fixed schedule and read [`TickInput`], a
//! snapshot of which [`Action`]s are held during the current simulation tick.
//! Edges (`just_pressed`) are computed in *tick* space, not frame space —
//! Bevy's `ButtonInput::just_pressed` is per rendered frame, so it would
//! double-fire at 30 FPS and vanish at 144 FPS relative to a 60 Hz tick.
//!
//! This is also the seed of the replay architecture: a recorded input trace
//! is nothing more than one [`Actions`] value per tick, fed to
//! [`TickInput::advance`] in place of the keyboard.

use bevy::input::InputSystems;
use bevy::prelude::*;

/// Everything the player can do. Order is significant: it defines the bit
/// layout of [`Actions`], which future trace files will store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Action {
    Left,
    Right,
    Up,
    Down,
    Jump,
    Restart,
    PrintInventory,
}

impl Action {
    pub const ALL: [Action; 7] = [
        Action::Left,
        Action::Right,
        Action::Up,
        Action::Down,
        Action::Jump,
        Action::Restart,
        Action::PrintInventory,
    ];

    /// Default keyboard binding. The only place keys are named.
    pub fn key(self) -> KeyCode {
        match self {
            Action::Left => KeyCode::KeyA,
            Action::Right => KeyCode::KeyD,
            Action::Up => KeyCode::KeyW,
            Action::Down => KeyCode::KeyS,
            Action::Jump => KeyCode::Space,
            Action::Restart => KeyCode::KeyR,
            Action::PrintInventory => KeyCode::KeyP,
        }
    }

    const fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// A set of actions, packed into one byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Actions(u8);

impl Actions {
    pub const NONE: Actions = Actions(0);

    pub fn contains(self, action: Action) -> bool {
        self.0 & action.bit() != 0
    }

    pub fn insert(&mut self, action: Action) {
        self.0 |= action.bit();
    }

    pub fn with(mut self, action: Action) -> Self {
        self.insert(action);
        self
    }

    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn from_bits(bits: u8) -> Self {
        Actions(bits)
    }
}

/// Input state for the current simulation tick. Read this from gameplay
/// systems instead of `ButtonInput<KeyCode>`.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct TickInput {
    held: Actions,
    prev: Actions,
    /// Number of ticks advanced so far (0 before the first tick).
    tick: u64,
}

impl TickInput {
    pub fn pressed(&self, action: Action) -> bool {
        self.held.contains(action)
    }

    /// True on the first tick an action is held.
    pub fn just_pressed(&self, action: Action) -> bool {
        self.held.contains(action) && !self.prev.contains(action)
    }

    pub fn held(&self) -> Actions {
        self.held
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Advance one tick with the given held set. Keyboard sampling calls
    /// this; so will trace replay.
    pub fn advance(&mut self, held: Actions) {
        self.prev = self.held;
        self.held = held;
        self.tick += 1;
    }
}

/// Frame-rate accumulator: remembers key presses that happened since the
/// last tick, so a tap shorter than one tick still registers.
#[derive(Resource, Default)]
struct PendingInput {
    pressed_since_tick: Actions,
}

fn accumulate_input(keys: Res<ButtonInput<KeyCode>>, mut pending: ResMut<PendingInput>) {
    for action in Action::ALL {
        if keys.just_pressed(action.key()) {
            pending.pressed_since_tick.insert(action);
        }
    }
}

fn sample_tick_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut pending: ResMut<PendingInput>,
    mut tick_input: ResMut<TickInput>,
) {
    let mut held = Actions::NONE;
    for action in Action::ALL {
        if keys.pressed(action.key()) || pending.pressed_since_tick.contains(action) {
            held.insert(action);
        }
    }
    pending.pressed_since_tick = Actions::NONE;
    tick_input.advance(held);
}

/// Samples the keyboard into [`TickInput`] once per fixed tick.
pub struct TickInputPlugin;

impl Plugin for TickInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TickInput>()
            .init_resource::<PendingInput>()
            .add_systems(PreUpdate, accumulate_input.after(InputSystems))
            .add_systems(FixedFirst, sample_tick_input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_are_computed_per_tick() {
        let mut input = TickInput::default();
        input.advance(Actions::NONE.with(Action::Jump));
        assert!(input.pressed(Action::Jump));
        assert!(input.just_pressed(Action::Jump));

        input.advance(Actions::NONE.with(Action::Jump));
        assert!(input.pressed(Action::Jump));
        assert!(!input.just_pressed(Action::Jump), "held, not newly pressed");

        input.advance(Actions::NONE);
        assert!(!input.pressed(Action::Jump));

        input.advance(Actions::NONE.with(Action::Jump));
        assert!(input.just_pressed(Action::Jump), "re-press after release");
        assert_eq!(input.tick(), 4);
    }

    #[test]
    fn actions_round_trip_through_bits() {
        let a = Actions::NONE.with(Action::Left).with(Action::Jump);
        assert_eq!(Actions::from_bits(a.bits()), a);
        assert!(a.contains(Action::Left) && a.contains(Action::Jump));
        assert!(!a.contains(Action::Right));
    }
}
