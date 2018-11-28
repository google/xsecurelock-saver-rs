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

//! Provides a specs ECS system for simple circle-based collision detection, for the Screensaver
//! Engine.
extern crate specs;
extern crate xsecurelock_saver;

#[cfg(feature = "debug-timing")]
use std::time::Instant;

use specs::{
    Component,
    Entities,
    Entity,
    Join,
    Read,
    ReadStorage,
    System,
    VecStorage,
    Write,
};

use xsecurelock_saver::engine::components::physics::{Position, Vector, Velocity};
use xsecurelock_saver::engine::components::scene::InScene;
use xsecurelock_saver::engine::resources::time::PhysicsDeltaTime;

/// A collision between two entities.
#[derive(Debug, Copy, Clone)]
pub struct CollisionEvent(pub Entity, pub Entity);

/// A resource that records the collisions that happened in the last frame.
#[derive(Debug, Default)]
pub struct LastUpdateCollisions(Vec<CollisionEvent>);

impl LastUpdateCollisions {
    pub fn iter(&self) -> ::std::iter::Cloned<::std::slice::Iter<CollisionEvent>> {
        self.0.iter().cloned()
    }
}

const NUM_LAYERS: usize = 32;

/// A collision layer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct CollisionLayer(usize);

impl CollisionLayer {
    pub fn new(layer: usize) -> Self {
        assert!(layer < NUM_LAYERS, format!("Max layer is {}", NUM_LAYERS));
        CollisionLayer(layer)
    }
}

/// A trait for checking which collision layers can collide with one another.
#[derive(Default)]
pub struct CollisionMatrix([u32; NUM_LAYERS]);

impl CollisionMatrix {
    /// Enables collision between the specified layers.
    #[inline]
    pub fn enable_collision(&mut self, l1: CollisionLayer, l2: CollisionLayer) {
        self.0[l1.0] |= 1 << l2.0;
        self.0[l2.0] |= 1 << l1.0;
    }

    #[inline]
    /// Disables collision between the specified layers.
    pub fn disable_collision(&mut self, l1: CollisionLayer, l2: CollisionLayer) {
        self.0[l1.0] &= !(1 << l2.0);
        self.0[l2.0] &= !(1 << l1.0);
    }

    #[inline]
    /// Returns true if the specified layers can collide.
    pub fn can_collide(&self, l1: CollisionLayer, l2: CollisionLayer) -> bool {
        (self.0[l1.0] | (1 << l2.0)) != 0
    }
}

/// A simple circle collider.
pub struct CircleCollider {
    /// The radius of the collider.
    pub radius: f32,
    /// The collision layer that this collider is in.
    pub layer: CollisionLayer,
}
impl Component for CircleCollider { 
    type Storage = VecStorage<Self>; 
}

impl CircleCollider {
    /// Create a new circle collider.
    pub fn new(radius: f32) -> Self {
        CircleCollider::new_in_layer(radius, Default::default())
    }

    /// Create a collider in the specified layer.
    pub fn new_in_layer(radius: f32, layer: CollisionLayer) -> Self {
        CircleCollider {
            radius,
            layer,
        }
    }
}

/// A system that calculates collisions through brute force. Only does discrete collisions, so may
/// miss fast moving objects.
#[derive(Default)]
pub struct BruteForceCollisionDetector;
impl<'a> System<'a> for BruteForceCollisionDetector {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, CircleCollider>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
        Read<'a, PhysicsDeltaTime>,
        Read<'a, CollisionMatrix>,
        Write<'a, LastUpdateCollisions>,
    );

    fn run(
        &mut self,
        (
            entities,
            colliders,
            positions,
            velocities,
            physics_delta,
            layers,
            mut collisions,
        ): Self::SystemData,
    ) {
        #[cfg(feature = "debug-timing")]
        let start = Instant::now();

        let pdt = physics_delta.0.as_seconds();
        collisions.0.clear();
        for (e1, c1, p1) in (&*entities, &colliders, &positions).join() {
            let v1 = velocities.get(e1).map_or(Vector::new(0., 0.), |v| v.linear);
            for (e2, c2, p2) in (&*entities, &colliders, &positions).join() {
                // Enforce ordering to ensure we only generate one collision per pair. We would
                // normally improve efficiency by starting the inner loop from e1.id() + 1, but
                // EntitiesRes doesn't seem to support that.
                // TODO(zstewart): Enforce ordering with a bitset maybe.
                if e2.id() <= e1.id() { continue; }
                if !layers.can_collide(c1.layer, c2.layer) { continue; }

                let v2 = velocities.get(e2).map_or(Vector::new(0., 0.), |v| v.linear);

                let netv = v1 - v2;
                let netv_len_sq = netv.norm_squared();

                let sep_sq = if netv_len_sq == 0. {
                    (p2.pos() - p1.pos()).norm_squared()
                } else {
                    let t = (p2.pos() - p1.pos()).dot(&netv) / netv_len_sq;
                    let t = if t < 0. { 0. } else if t > pdt { pdt } else { t };
                    let projected = p1.pos() + t * netv;
                    (projected - p2.pos()).norm_squared()
                };
                let total_size = c1.radius + c2.radius;
                let size_sq = total_size * total_size;
                if sep_sq <= size_sq {
                    collisions.0.push(CollisionEvent(e1, e2));
                }
            }
        }

        #[cfg(feature = "debug-timing")] {
            let elapsed = start.elapsed();
            println!("Calculating Collisions took: {}s, {}µs", elapsed.as_secs(), elapsed.subsec_micros());
        }
    }
}

/// System that can be added to the scene change step to clear all collisions involving entities
/// with InScene.
pub struct ClearCollisionsInvolvingSceneEntities;
impl<'a> System<'a> for ClearCollisionsInvolvingSceneEntities {
    type SystemData = (
        ReadStorage<'a, InScene>,
        Write<'a, LastUpdateCollisions>,
    );

    fn run(&mut self, (scene_markers, mut collisions): Self::SystemData) {
        collisions.0.retain(|collision| {
            scene_markers.get(collision.0).is_none()
                && scene_markers.get(collision.1).is_none()
        });
    }
}
