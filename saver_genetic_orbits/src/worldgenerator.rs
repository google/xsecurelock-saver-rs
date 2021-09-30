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

use std::time::Duration;

use bevy::ecs::component::Component;
use bevy::prelude::*;
use rand_distr::{Bernoulli, Distribution, Exp, Normal, Uniform};

use crate::config::generator::{
    GeneratorConfig, MutationParameters, NewPlanetParameters, NewWorldParameters,
    PlanetMutationParameters,
};
use crate::config::util::{
    Distribution as ConfDist, ExponentialDistribution, NormalDistribution, UniformDistribution,
};
use crate::model::{Planet, Scenario, World};
use crate::statustracker::ActiveWorld;
use crate::storage::sqlite::SqliteStorage;
use crate::storage::Storage;

use super::SaverState;

/// Configures the world generator.
pub struct WorldGeneratorPlugin;

impl Plugin for WorldGeneratorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(DelayResume(Timer::new(Duration::from_millis(500), false)))
            .add_system_set(
                SystemSet::on_enter(SaverState::Generate)
                    .with_system(generate_world::<SqliteStorage>.system()),
            )
            .add_system_set(
                SystemSet::on_update(SaverState::Generate).with_system(resume.system()),
            );
    }
}

/// Generates a new world to run and inserts it into ActiveWorld, then sets the state to Run.
fn generate_world<S: Storage + Component>(
    config: Res<GeneratorConfig>,
    mut storage: ResMut<S>,
    mut scenario: ResMut<ActiveWorld>,
    mut resume: ResMut<DelayResume>,
) {
    info!("Generating world");
    let parent = pick_parent(&mut *storage, config.create_new_scenario_probability);

    let world = match parent {
        Some(ref parent) => generate_child_world(&parent.world, &config.mutation_parameters),
        None => generate_new_world(&config.new_world_parameters),
    };

    scenario.start(world, parent);

    resume.0.reset();
}

struct DelayResume(Timer);

/// Delays returning to run by half a second.
fn resume(mut state: ResMut<State<SaverState>>, mut timer: ResMut<DelayResume>, time: Res<Time>) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        if let Err(err) = state.set(SaverState::Run) {
            warn!("Failed to switch from generate to run: {:?}", err);
        }
    }
}

/// Picks a scenario to mutate or None if a new scenario should be generated.
fn pick_parent(
    storage: &mut impl Storage,
    create_new_scenario_probability: f64,
) -> Option<Scenario> {
    let num_scenarios = match storage.num_scenarios() {
        Ok(0) => {
            info!("No existing scenarios to mutate, generating new one by default");
            return None;
        }
        Ok(ns) => ns,
        Err(err) => {
            error!("Error getting number of scenarios: {}", err);
            return None;
        }
    };
    let picked_scenario = select_index(num_scenarios, create_new_scenario_probability);
    match storage.get_nth_scenario_by_score(picked_scenario) {
        Ok(Some(scenario)) => {
            info!(
                "Mutating Scenario {} (parent: {:?}, family: {}, generation: {}, score: {}, \
                planets: {})",
                scenario.id,
                scenario.parent,
                scenario.family,
                scenario.generation,
                scenario.score,
                scenario.world.planets.len(),
            );
            Some(scenario)
        }
        Ok(None) => {
            info!("Generating new Scenario");
            None
        }
        Err(err) => {
            error!(
                "Generating new Scenario because of error fetching scenario {}: {}",
                picked_scenario, err,
            );
            None
        }
    }
}

/// Selects a random index from the number of scenarios. The selected index may be out of
/// range.  Uses an exponential distribution where the probability of choosing an out of range
/// index (and thus starting a new scenario) is given by the config.
fn select_index(num_items: u64, create_new_scenario_probability: f64) -> u64 {
    assert!(num_items > 0);
    // The CDF of the exponential distribution is f(x) = 1-e^(-lx). In order to have
    // P probability of getting a value in-range, we want to choose l such that
    // f(num-scenarios) = P. Therefore we solve for l:
    // l = -ln(1 - P) / num-scenarios
    let lambda = -(create_new_scenario_probability.ln()) / num_items as f64;
    let dist = Exp::new(lambda).unwrap();
    dist.sample(&mut rand::thread_rng()) as u64
}

/// Randomly generate a new world.
fn generate_new_world(params: &NewWorldParameters) -> World {
    let num_planets = match params.num_planets_dist {
        ConfDist::Exponential(ExponentialDistribution(lambda)) => {
            Exp::new(lambda).unwrap().sample(&mut rand::thread_rng()) as usize
        }
        ConfDist::Normal(NormalDistribution {
            mean,
            standard_deviation,
        }) => Normal::new(mean, standard_deviation)
            .unwrap()
            .sample(&mut rand::thread_rng())
            .round() as usize,
        ConfDist::Uniform(UniformDistribution { min, max }) => {
            Uniform::new_inclusive(min as usize, max as usize).sample(&mut rand::thread_rng())
        }
    };
    let num_planets = params.num_planets_range.clamp_inclusive(num_planets);
    info!("Generating {} planets", num_planets);

    let mut planets = Vec::with_capacity(num_planets);
    for _ in 0..num_planets {
        planets.push(generate_new_planet(&params.planet_parameters));
    }

    let mut world = World { planets };
    world.merge_overlapping_planets();
    info!(
        "After overlap cleanup, world had {} planets",
        world.planets.len()
    );
    world
}

/// Mutate the given parent world to generate a new random world.
fn generate_child_world(parent: &World, params: &MutationParameters) -> World {
    let num_planets_to_add = match params.add_planets_dist {
        ConfDist::Exponential(ExponentialDistribution(lambda)) => {
            Exp::new(lambda).unwrap().sample(&mut rand::thread_rng()) as usize
        }
        ConfDist::Normal(NormalDistribution {
            mean,
            standard_deviation,
        }) => Normal::new(mean, standard_deviation)
            .unwrap()
            .sample(&mut rand::thread_rng())
            .round() as usize,
        ConfDist::Uniform(UniformDistribution { min, max }) => {
            Uniform::new_inclusive(min as usize, max as usize).sample(&mut rand::thread_rng())
        }
    };
    let num_planets_to_add = params
        .add_planets_limits
        .clamp_inclusive(num_planets_to_add);

    let num_planets_to_remove = match params.remove_planets_dist {
        ConfDist::Exponential(ExponentialDistribution(lambda)) => {
            Exp::new(lambda).unwrap().sample(&mut rand::thread_rng()) as usize
        }
        ConfDist::Normal(NormalDistribution {
            mean,
            standard_deviation,
        }) => Normal::new(mean, standard_deviation)
            .unwrap()
            .sample(&mut rand::thread_rng())
            .round() as usize,
        ConfDist::Uniform(UniformDistribution { min, max }) => {
            Uniform::new_inclusive(min as usize, max as usize).sample(&mut rand::thread_rng())
        }
    };
    let num_planets_to_remove = params
        .remove_planets_limits
        .clamp_inclusive(num_planets_to_remove);
    let num_planets_to_remove = parent.planets.len().min(num_planets_to_remove);

    let change_planet_dist = Bernoulli::new(params.fraction_of_planets_to_change).unwrap();

    // Order of changes is remove, modify, add. This is so we don't remove or modify newly
    // added planets and don't modify planets that are about to be removed.

    let mut world = parent.clone();

    // Remove:
    for _ in 0..num_planets_to_remove {
        // panics if start >= end, but this loop doesn't run if planets.len() == 0, so this is
        // safe.
        let selected = Uniform::new(0, world.planets.len()).sample(&mut rand::thread_rng());
        world.planets.remove(selected);
    }
    info!("Removed {} planets", num_planets_to_remove);

    // Modify
    let mut num_modified = 0;
    for planet in world.planets.iter_mut() {
        if change_planet_dist.sample(&mut rand::thread_rng()) {
            mutate_planet(planet, &params.planet_mutation_parameters);
            num_modified += 1;
        }
    }
    info!("Modified {} planets", num_modified);

    for _ in 0..num_planets_to_add {
        world
            .planets
            .push(generate_new_planet(&params.new_planet_parameters));
    }
    info!("Added {} planets", num_planets_to_add);

    world.merge_overlapping_planets();
    info!(
        "After overlap cleanup, world had {} planets",
        world.planets.len()
    );
    world
}

/// Generates a new randomly sized planet at a random location with random velocity.
fn generate_new_planet(params: &NewPlanetParameters) -> Planet {
    let x_dist = Uniform::new_inclusive(params.start_position.x.min, params.start_position.x.max);
    let y_dist = Uniform::new_inclusive(params.start_position.y.min, params.start_position.y.max);
    let z_dist = Uniform::new_inclusive(params.start_position.z.min, params.start_position.z.max);

    let position = Vec3::new(
        x_dist.sample(&mut rand::thread_rng()) as f32,
        y_dist.sample(&mut rand::thread_rng()) as f32,
        z_dist.sample(&mut rand::thread_rng()) as f32,
    );

    let x_velocity_dist = Normal::new(
        params.start_velocity.x.mean,
        params.start_velocity.x.standard_deviation,
    )
    .unwrap();
    let y_velocity_dist = Normal::new(
        params.start_velocity.y.mean,
        params.start_velocity.y.standard_deviation,
    )
    .unwrap();
    let z_velocity_dist = Normal::new(
        params.start_velocity.z.mean,
        params.start_velocity.z.standard_deviation,
    )
    .unwrap();

    let velocity = Vec3::new(
        x_velocity_dist.sample(&mut rand::thread_rng()) as f32,
        y_velocity_dist.sample(&mut rand::thread_rng()) as f32,
        z_velocity_dist.sample(&mut rand::thread_rng()) as f32,
    );

    let mass_dist =
        Normal::new(params.start_mass.mean, params.start_mass.standard_deviation).unwrap();
    let mass = params
        .min_start_mass
        .max(mass_dist.sample(&mut rand::thread_rng()) as f32);

    Planet {
        position,
        velocity,
        mass,
    }
}

/// Mutates a planet by making small changes to the mass, position, and velocity.
fn mutate_planet(planet: &mut Planet, params: &PlanetMutationParameters) {
    let x_pos_change = Normal::new(
        params.position_change.x.mean,
        params.position_change.x.standard_deviation,
    )
    .unwrap()
    .sample(&mut rand::thread_rng()) as f32;
    let y_pos_change = Normal::new(
        params.position_change.y.mean,
        params.position_change.y.standard_deviation,
    )
    .unwrap()
    .sample(&mut rand::thread_rng()) as f32;
    let z_pos_change = Normal::new(
        params.position_change.z.mean,
        params.position_change.z.standard_deviation,
    )
    .unwrap()
    .sample(&mut rand::thread_rng()) as f32;

    let x_vel_change = Normal::new(
        params.velocity_change.x.mean,
        params.velocity_change.x.standard_deviation,
    )
    .unwrap()
    .sample(&mut rand::thread_rng()) as f32;
    let y_vel_change = Normal::new(
        params.velocity_change.y.mean,
        params.velocity_change.y.standard_deviation,
    )
    .unwrap()
    .sample(&mut rand::thread_rng()) as f32;
    let z_vel_change = Normal::new(
        params.velocity_change.z.mean,
        params.velocity_change.z.standard_deviation,
    )
    .unwrap()
    .sample(&mut rand::thread_rng()) as f32;

    let mass_change = match params.mass_change {
        ConfDist::Exponential(ExponentialDistribution(lambda)) => {
            Exp::new(lambda).unwrap().sample(&mut rand::thread_rng())
        }
        ConfDist::Normal(NormalDistribution {
            mean,
            standard_deviation,
        }) => Normal::new(mean, standard_deviation)
            .unwrap()
            .sample(&mut rand::thread_rng()),
        ConfDist::Uniform(UniformDistribution { min, max }) => {
            Uniform::new_inclusive(min, max).sample(&mut rand::thread_rng())
        }
    } as f32;

    planet.position.x += x_pos_change;
    planet.position.y += y_pos_change;
    planet.position.z += z_pos_change;
    planet.velocity.x += x_vel_change;
    planet.velocity.y += y_vel_change;
    planet.velocity.z += z_vel_change;
    planet.mass += mass_change;
    planet.mass = params.min_mass.max(planet.mass);
}
