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

//! Provides a specs ECS system for simple gravity calculations for the Screensaver Engine.
extern crate specs;
extern crate xsecurelock_saver;

#[cfg(feature = "debug-timing")]
use std::time::Instant;

use specs::{
    Component,
    Entities,
    Join,
    NullStorage,
    Read,
    ReadStorage,
    System,
    WriteStorage,
};

use xsecurelock_saver::engine::components::physics::{
    ForceAccumulator,
    Position,
    Mass,
};

pub struct GravitationalConstant(pub f32);

impl Default for GravitationalConstant {
    fn default() -> Self { GravitationalConstant(1.) }
}

/// Marker Component for objects that other objects are gravitationally attracted to.
#[derive(Default)]
pub struct GravitySource;

impl Component for GravitySource {
    type Storage = NullStorage<Self>;
}

/// Marker Component for objects that are affected by gravity.
#[derive(Default)]
pub struct GravityTarget;

impl Component for GravityTarget {
    type Storage = NullStorage<Self>;
}

/// System that applies gravity forces to physics objects.
///
/// Uses a simple N^2 implementation. This is based on the assumption that few objects will be
/// gravity sources while many will be gravity targets, in which case this implementation should be
/// fast.
///
/// mass on gravity sources and targets is optional and will use the default of 1 if unavailable.
pub struct GravitySystem;
impl<'a> System<'a> for GravitySystem {
    type SystemData = (
        Read<'a, GravitationalConstant>,
        Entities<'a>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Mass>,
        ReadStorage<'a, GravitySource>,
        ReadStorage<'a, GravityTarget>,
        WriteStorage<'a, ForceAccumulator>,
    );

    fn run(
        &mut self,
        (g, entities, positions, masses, sources, targets, mut forces): Self::SystemData,
    ) {
        #[cfg(feature = "debug-timing")]
        let start = Instant::now();

        for (ent, pos, _, acc) in (&*entities, &positions, &targets, &mut forces).join() {
            let mass = masses.get(ent).cloned().unwrap_or_default().linear;
            for (source_ent, source_pos, _) in (&*entities, &positions, &sources).join() {
                if ent == source_ent { continue; }
                let source_mass = masses.get(source_ent).cloned().unwrap_or_default().linear;
                let force_dir = source_pos.pos() - pos.pos();
                let norm = force_dir.norm();
                let norm3 = norm * norm * norm;
                let force = g.0 * mass * source_mass * force_dir / norm3;
                acc.linear += force;
            }
        }

        #[cfg(feature = "debug-timing")] {
            let elapsed = start.elapsed();
            println!("Calculating Gravity took: {}s, {}µs", elapsed.as_secs(), elapsed.subsec_micros());
        }
    }
}
