// Copyright 2018-2021 Google LLC
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

use bevy::prelude::shape;
use bevy::prelude::*;
use bevy::render::camera::PerspectiveProjection;
use bevy_rapier3d::na::{Point3, Vector3};
use bevy_rapier3d::prelude::*;
use rand_distr::{Distribution, Uniform};

use crate::config::camera::CameraConfig;
use crate::model::Planet as PlanetConfig;
use crate::statustracker::ActiveWorld;
use crate::SaverState;

/// Plugin handles configuring and executing the world simulation.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PlanetMesh>()
            .add_startup_system(setup_camera_light.system())
            .add_startup_system(remove_rapier_gravity.system())
            .add_system(rotate_camera.system())
            .add_system_set(
                SystemSet::on_enter(SaverState::Run)
                    .with_system(remove_planets.system().label("remove-old"))
                    .with_system(spawn_planets.system().after("remove-old")),
            )
            .add_system(gravity.system());
    }
}

/// Disables the default rapier gravity.
fn remove_rapier_gravity(mut rcfg: ResMut<RapierConfiguration>) {
    rcfg.gravity = Vector3::zeros();
}

/// Add a light and a camera.
fn setup_camera_light(mut commands: Commands) {
    // light
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        light: Light {
            depth: 0.1..50_000.0,
            range: 10_000.0,
            intensity: 10_000_000.0,
            ..Default::default()
        },
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        perspective_projection: PerspectiveProjection {
            near: 1.0,
            far: 20_000.0,
            ..Default::default()
        },
        ..Default::default()
    });
}

/// rotate the camera around the origin.
fn rotate_camera(
    mut query: Query<&mut Transform, With<PerspectiveProjection>>,
    time: Res<Time>,
    config: Res<CameraConfig>,
) {
    let t = time.seconds_since_startup() as f32 * config.rotation_speed;
    for mut camera in query.iter_mut() {
        *camera = Transform::from_xyz(t.sin() * config.view_dist, 0.0, t.cos() * config.view_dist)
            .looking_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Holds the sphere mesh used to render planets.
struct PlanetMesh(Handle<Mesh>);

impl FromWorld for PlanetMesh {
    fn from_world(world: &mut World) -> Self {
        let mesh = world
            .get_resource_mut::<Assets<Mesh>>()
            .unwrap()
            .add(Mesh::from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 2,
            }));
        Self(mesh)
    }
}

/// Marker component to identify planets for scoring and deletion.
#[derive(Default)]
pub struct Planet;

/// Marker to apply gravity.
#[derive(Default)]
struct ApplyGravity;

#[derive(Bundle, Default)]
struct PlanetBundle {
    #[bundle]
    pbr: PbrBundle,
    #[bundle]
    rigidbody: RigidBodyBundle,
    #[bundle]
    collider: ColliderBundle,
    sync: RigidBodyPositionSync,
    gravity: ApplyGravity,
    planet: Planet,
}

impl PlanetBundle {
    fn new_from_planet(
        planet: &PlanetConfig,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
    ) -> Self {
        let radius = planet.radius();
        Self {
            pbr: PbrBundle {
                mesh,
                material,
                transform: Transform {
                    translation: planet.position,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(radius, radius, radius),
                },
                ..Default::default()
            },
            rigidbody: RigidBodyBundle {
                position: planet.position.into(),
                velocity: RigidBodyVelocity {
                    linvel: planet.velocity.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            collider: ColliderBundle {
                shape: ColliderShape::ball(radius),
                mass_properties: ColliderMassProps::Density(PlanetConfig::DENSITY),
                ..Default::default()
            },
            sync: RigidBodyPositionSync::Interpolated { prev_pos: None },
            ..Default::default()
        }
    }
}

/// Generates a random color, usually fairly bright.
fn generate_random_color() -> Color {
    let hue_dist = Uniform::new(0.0, 360.0);
    let sat_dist = Uniform::new_inclusive(0.75, 1.0);
    let lightness_dist = Uniform::new_inclusive(0.75, 1.0);

    let h = hue_dist.sample(&mut rand::thread_rng());
    let s = sat_dist.sample(&mut rand::thread_rng());
    let l = lightness_dist.sample(&mut rand::thread_rng());
    Color::hsl(h, s, l)
}

fn spawn_planets(
    mut commands: Commands,
    world: Res<ActiveWorld>,
    mesh: Res<PlanetMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for planet in &world.world.planets {
        let material = materials.add(generate_random_color().into());
        commands.spawn_bundle(PlanetBundle::new_from_planet(
            planet,
            mesh.0.clone(),
            material,
        ));
    }
}

/// Removes all planets.
fn remove_planets(mut commands: Commands, query: Query<Entity, With<Planet>>) {
    for planet in query.iter() {
        commands.entity(planet).despawn();
    }
}

/// Intermediate accumulator for gravity calculations.
struct Accumulator {
    /// Center of mass of the rigidbody.
    com: Point3<f32>,
    /// Mass of the rigidbody.
    mass: f32,
    /// Accumulated forces.
    force: Vector3<f32>,
}

/// Aplies gravity to rigidbodies.
fn gravity(
    mut accumulator: Local<Vec<Accumulator>>,
    mut query: Query<(&RigidBodyMassProps, &mut RigidBodyForces), With<ApplyGravity>>,
) {
    const G: f32 = 500.0;

    accumulator.clear();
    for (mass, _) in query.iter_mut() {
        accumulator.push(Accumulator {
            com: mass.world_com,
            mass: mass.mass(),
            force: Vector3::zeros(),
        });
    }
    for i in 1..accumulator.len() {
        let (current, rest) = accumulator.split_at_mut(i);
        let current = &mut current[i - 1];
        for other in rest {
            let diff = other.com - current.com;
            let force_magnitude = G * current.mass * other.mass / diff.norm_squared();
            if !force_magnitude.is_finite() {
                continue;
            }
            let force_dir = diff.normalize();
            let force = force_magnitude * force_dir;
            current.force += force;
            other.force -= force;
        }
    }
    for ((_, mut force), acc) in query.iter_mut().zip(&*accumulator) {
        force.force += acc.force;
    }
}
