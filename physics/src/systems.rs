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
    Entities,
    Join,
    Read,
    ReadStorage,
    System,
    WriteStorage,
};

use crate::{
    components::{
        ForceAccumulator,
        Position,
        Mass,
        Velocity,
    },
    resources::PhysicsDeltaTime,
};

/// Copies current positions to previous positions to allow interpolation. Should run before each
/// physics tick.
pub struct SetupNextPhysicsPosition;

impl<'a> System<'a> for SetupNextPhysicsPosition {
    type SystemData = WriteStorage<'a, Position>;

    fn run(&mut self, mut positions: Self::SystemData) {
        for pos in (&mut positions).join() {
            pos.step_previous();
        }
    }
}

pub struct ClearForceAccumulators;

impl<'a> System<'a> for ClearForceAccumulators {
    type SystemData = WriteStorage<'a, ForceAccumulator>;

    fn run(&mut self, mut accumulators: Self::SystemData) {
        for acc in (&mut accumulators).join() {
            acc.clear();
        }
    }
}

/// Integrates forces, adding to the velocity, multiplied by PhysicsDeltaTime.
pub struct SympleticEulerForceStep;
impl<'a> System<'a> for SympleticEulerForceStep {
    type SystemData = (
        Read<'a, PhysicsDeltaTime>,
        Entities<'a>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Mass>,
        ReadStorage<'a, ForceAccumulator>,
    );

    fn run(&mut self, (time, entities, mut velocities, masses, forces): Self::SystemData) {
        let dt = time.0.as_seconds();
        for (vel, force, ent) in (&mut velocities, &forces, &*entities).join() {
            let mass = masses.get(ent).cloned().unwrap_or_default();
            vel.linear += (force.linear / mass.linear) * dt;
            vel.angular *= force.angular.powf(1./mass.angular).powf(dt);
        }
    }
}

/// Integrates velocities, adding to the position, multiplied by PhysicsDeltaTime.
pub struct SympleticEulerVelocityStep;
impl<'a> System<'a> for SympleticEulerVelocityStep {
    type SystemData = (
        Read<'a, PhysicsDeltaTime>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
    );

    fn run(&mut self, (time, mut positions, velocities): Self::SystemData) {
        let dt = time.0.as_seconds();
        for (pos, vel) in (&mut positions, &velocities).join() {
            *pos.pos_mut() += vel.linear * dt;
            *pos.rot_mut() *= vel.angular.powf(dt);
        }
    }
}
