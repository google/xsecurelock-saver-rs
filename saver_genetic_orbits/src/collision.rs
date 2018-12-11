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

use sfml::graphics::Color;
use sfml::system::{Vector2f, Vector3f};

use specs::{
    Component,
    Entities,
    Entity,
    Join,
    Read,
    ReadStorage,
    System,
    VecStorage,
    WriteStorage,
};

use physics::components::{
    ForceAccumulator,
    Mass,
    Position,
    Rotation,
    Vector,
    Velocity,
};

use xsecurelock_saver::engine::components::{
    delete::Deleted,
    draw::{
        DrawColor,
        DrawShape,
        ShapeType,
    },
};

use circle_collision::{
    CircleCollider,
    CollisionEvent,
    CollisionLayer,
    LastUpdateCollisions,
};

use crate::model::Planet;

#[inline] pub fn planet() -> CollisionLayer { CollisionLayer::new(1) }

pub struct MergeCollidedPlanets;

impl MergeCollidedPlanets {
    fn extract_properties<'a>(
        ent: Entity, 
        positions: &WriteStorage<'a, Position>,
        velocities: &WriteStorage<'a, Velocity>,
        masses: &WriteStorage<'a, Mass>,
        colors: &WriteStorage<'a, DrawColor>,
        forces: &WriteStorage<'a, ForceAccumulator>,
    ) -> Option<(Vector, Vector, f32, Vector3f, Vector)> {
        let pos = match positions.get(ent) {
            Some(pos) => pos.pos(),
            None => return None,
        };
        let vel = match velocities.get(ent) {
            Some(vel) => vel.linear,
            None => return None,
        };
        let mass = match masses.get(ent) {
            Some(mass) => mass.linear,
            None => return None,
        };
        let color = match colors.get(ent) {
            Some(color) => Self::vectorize_color(color.fill_color),
            None => return None,
        };
        let force = match forces.get(ent) {
            Some(force) => force.linear,
            None => return None,
        };
        Some((pos, vel, mass, color, force))
    }

    fn vectorize_color(color: Color) -> Vector3f {
        Vector3f::new(
            color.r as f32 / 255.,
            color.g as f32 / 255.,
            color.b as f32 / 255.,
        )
    }

    fn colorize_vector(color: Vector3f) -> Color {
        Color::rgb(
            (color.x * 255.).round() as u8,
            (color.y * 255.).round() as u8,
            (color.z * 255.).round() as u8,
        )
    }
}

impl<'a> System<'a> for MergeCollidedPlanets {
    type SystemData = (
        Entities<'a>,
        Read<'a, LastUpdateCollisions>,
        WriteStorage<'a, MergedInto>,
        WriteStorage<'a, DrawColor>,
        WriteStorage<'a, DrawShape>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Mass>,
        WriteStorage<'a, ForceAccumulator>,
        WriteStorage<'a, CircleCollider>,
    );

    fn run(
        &mut self,
        (
            entities,
            collisions,
            mut successors,
            mut colors,
            mut shapes,
            mut positions,
            mut velocities,
            mut masses,
            mut forces,
            mut colliders,
        ): Self::SystemData,
    ) {
        for CollisionEvent(mut e1, mut e2) in collisions.iter() {
            while let Some(MergedInto(e1successor)) = successors.get(e1) {
                e1 = *e1successor;
            }
            while let Some(MergedInto(e2successor)) = successors.get(e2) {
                e2 = *e2successor;
            }
            if !entities.is_alive(e1) || !entities.is_alive(e2) {
                warn!("Collision between dead entities!");
                continue;
            }
            if e1 == e2 {
                // Previous merges have already combined these two planets.
                continue;
            }
            let e1props = Self::extract_properties(
                e1, &positions, &velocities, &masses, &colors, &forces,
            );
            let e2props = Self::extract_properties(
                e2, &positions, &velocities, &masses, &colors, &forces,
            );
            let (p1, v1, m1, c1, f1) = match e1props {
                Some(props) => props,
                None => {
                    warn!("Found entitiy missing some properties to be a planet");
                    continue;
                },
            };
            let (p2, v2, m2, c2, f2) = match e2props {
                Some(props) => props,
                None => {
                    warn!("Found entitiy missing some properties to be a planet");
                    continue;
                },
            };
            let total_mass = m1 + m2;
            let e1fract = m1 / total_mass;
            let e2fract = m2 / total_mass;
            let pos = p1 * e1fract + p2 * e2fract;
            let vel = v1 * e1fract + v2 * e2fract;
            let color = Self::colorize_vector(c1 * e1fract + c2 * e2fract);
            let force = f1 + f2;
            let radius = Planet::radius_from_mass(total_mass);

            successors.insert(e2, MergedInto(e1)).unwrap();
            // Reinsert components with the new properties:
            // Drawing:
            colors.insert(e1, DrawColor {
                fill_color: color,
                outline_color: color,
                outline_thickness: 0.,
            }).unwrap();
            shapes.insert(e1, DrawShape {
                shape_type: ShapeType::Circle {
                    radius,
                    point_count: crate::worldgenerator::radius_to_point_count(radius),
                },
                origin: Vector2f::new(radius, radius),
            }).unwrap();
            // Physics:
            positions.insert(e1, Position::new(pos, Rotation::from_angle(0.))).unwrap();
            velocities.insert(e1, Velocity {
                linear: vel,
                angular: Rotation::from_angle(0.),
            }).unwrap();
            masses.insert(e1, Mass {
                linear: total_mass,
                angular: 1.,
            }).unwrap();
            forces.insert(e1, ForceAccumulator {
                linear: force,
                angular: Rotation::from_angle(0.),
            }).unwrap();
            colliders.insert(e1, CircleCollider::new_in_layer(radius, planet())).unwrap();
        }
    }
}

pub struct DeleteCollidedPlanets;
impl<'a> System<'a> for DeleteCollidedPlanets {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, MergedInto>,
        WriteStorage<'a, Deleted>,
    );

    fn run(&mut self, (entities, successors, mut deleted): Self::SystemData) {
        for (ent, _) in (&*entities, &successors).join() {
            deleted.insert(ent, Deleted).unwrap();
        }
    }
}

pub struct MergedInto(Entity);
impl Component for MergedInto { type Storage = VecStorage<Self>; }
