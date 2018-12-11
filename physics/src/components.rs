// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use specs::{
    Component,
    VecStorage,
    World,
};

use crate::{
    resources::PhysicsElapsed,
    time::Time,
};

pub type Vector = ::nalgebra::Vector2<f32>;
pub type Isometry = ::nalgebra::Isometry2<f32>;
pub type Translation = ::nalgebra::Translation2<f32>;
pub type Rotation = ::nalgebra::UnitComplex<f32>;

pub(crate) fn register_all(world: &mut World) {
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Mass>();
    world.register::<ForceAccumulator>();
}

/// How position interpolation is handled.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PositionInterpolationMode {
    /// Position is interpolated.
    Interpolated,
    /// Positions are not interpolated.
    Static,
}

/// The physics system position. Should usually only be updated during physics update.
#[derive(Debug)]
pub struct Position {
    mode: PositionInterpolationMode,
    current: (Vector, Rotation),
    previous: (Vector, Rotation),
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

impl Position {
    /// Creates an interpolated position with the specified starting position and rotation.
    pub fn new(pos: Vector, rot: Rotation) -> Self {
        Self {
            mode: PositionInterpolationMode::Interpolated,
            current: (pos, rot),
            previous: (pos, rot),
        }
    }

    /// Creates a position with the specified starting position and rotation, and the given
    /// interpolation mode.
    pub fn new_with_mode(pos: Vector, rot: Rotation, mode: PositionInterpolationMode) -> Self {
        Self {
            mode,
            current: (pos, rot),
            previous: (pos, rot),
        }
    }

    /// The position of the object.
    #[inline]
    pub fn pos(&self) -> Vector { self.current.0 }

    /// Mutable access to the position of the object.
    #[inline]
    pub fn pos_mut(&mut self) -> &mut Vector { &mut self.current.0 }

    /// Sets the postion of the object.
    #[inline]
    pub fn set_pos(&mut self, p: Vector) { self.current.0 = p }

    /// Sets the position, clearing the previous position. This allows moving an interpolated
    /// position without interpolation for a single frame.
    #[inline]
    pub fn teleport_pos(&mut self, p: Vector) {
        self.current.0 = p;
        self.previous.0 = p;
    }

    /// The rotation of the object.
    #[inline]
    pub fn rot(&self) -> Rotation { self.current.1 }

    /// Mutable access to the rotation of the object.
    #[inline]
    pub fn rot_mut(&mut self) -> &mut Rotation { &mut self.current.1 }

    /// Sets the rotation of the object.
    #[inline]
    pub fn set_rot(&mut self, r: Rotation) { self.current.1 = r; }


    /// Sets the rotation, clearing the previous rotation. This allows moving an interpolated
    /// rotation without interpolation for a single frame.
    #[inline]
    pub fn teleport_rot(&mut self, r: Rotation) {
        self.current.1 = r;
        self.previous.1 = r;
    }

    /// Gets the interpolation mode.
    #[inline]
    pub fn mode(&self) -> PositionInterpolationMode { self.mode }

    /// Sets the interpolation mode.
    #[inline]
    pub fn set_mode(&mut self, mode: PositionInterpolationMode) { self.mode = mode; }

    /// Interpolate from the previous position to the current position based on the relative
    /// physics and real times.
    pub fn interpolate(
        &self, real_time: Time, physics_time: &PhysicsElapsed,
    ) -> (Vector, Rotation) {
        let factor = Self::physics_interpolation_factor(real_time, physics_time);
        match self.mode {
            PositionInterpolationMode::Interpolated => {
                let pos = self.current.0 * factor + self.previous.0 * (1. - factor);
                let rotation = self.previous.1.rotation_to(&self.current.1).powf(factor);
                (pos, self.previous.1 * rotation)
            },
            PositionInterpolationMode::Static => self.current,
        }
    }

    /// Calculate an interpolation factor from the previous physics timestep to now.
    fn physics_interpolation_factor(elapsed: Time, physics: &PhysicsElapsed) -> f32 {
        let since_physics = elapsed - physics.previous;
        let physics_step = physics.current - physics.previous;
        since_physics.as_seconds() / physics_step.as_seconds()
    }

    /// Copy current to previous to setup for the next iteration.
    pub(crate) fn step_previous(&mut self) {
        self.previous = self.current;
    }
}

/// Velocity of an object, used for Sympletic Euler integration.
pub struct Velocity {
    pub linear: Vector,
    pub angular: Rotation,
}

impl Component for Velocity {
    type Storage = VecStorage<Self>;
}

impl Velocity {
    /// Create a new velocity component.
    pub fn new(linear: Vector, angular: f32) -> Self {
        Velocity {
            linear,
            angular: Rotation::from_angle(angular),
        }
    }
}

/// The mass of an object.
#[derive(Copy, Clone)]
pub struct Mass {
    /// Linear inertia of this object.
    pub linear: f32,
    /// Angular (moment of inertia) of this object.
    pub angular: f32,
}

impl Component for Mass {
    type Storage = VecStorage<Self>;
}

impl Default for Mass {
    fn default() -> Self {
        Self {
            linear: 1.,
            angular: 1.,
        }
    }
}

/// Accumulates forces applied to an object over the course of a frame.
pub struct ForceAccumulator {
    /// The linear force applied to an object.
    pub linear: Vector,
    /// The torque applied to an object.
    pub angular: Rotation,
}

impl Component for ForceAccumulator {
    type Storage = VecStorage<Self>;
}

impl ForceAccumulator {
    pub fn clear(&mut self) {
        *self = Default::default();
    }
}

impl Default for ForceAccumulator {
    /// Create an empty force accumulator.
    fn default() -> Self {
        Self {
            linear: Vector::new(0., 0.),
            angular: Rotation::from_angle(0.),
        }
    }
}
