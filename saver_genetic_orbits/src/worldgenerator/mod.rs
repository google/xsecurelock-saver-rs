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

use std::marker::PhantomData;

use sfml::graphics::Color;
use sfml::system::Vector2f;

use specs::{
    Entities,
    LazyUpdate,
    Read,
    Write,
};
use rand::{
    Rng,
    distributions::{
        Bernoulli,
        Distribution,
        Exp,
        Normal,
        Uniform,
    },
};

use xsecurelock_saver::engine::{
    resources::scene::SceneLoader,
    components::{
        draw::{
            DrawColor,
            DrawShape,
            ShapeType,
        },
        scene::InScene,
        physics::{
            ForceAccumulator,
            Mass,
            Position,
            Rotation,
            Vector,
            Velocity,
        },
    },
};

use circle_collision::CircleCollider;
use gravity::{GravitySource, GravityTarget};

use collision;
use model::{Planet, Scenario, World};
use statustracker;
use statustracker::ActiveWorld;
use storage::Storage;
use self::per_thread_rng::PerThreadRng;

pub mod per_thread_rng;

pub struct WorldGenerator<T, R = PerThreadRng> {
    rng: R,
    _phantom_storage_type: PhantomData<T>,
}

impl<T, R> WorldGenerator<T, R> {
    /// Constructs a new WorldGenerator with the given RNG.
    #[allow(dead_code)]
    fn new(rng: R) -> Self {
        WorldGenerator {
            rng,
            _phantom_storage_type: PhantomData,
        }
    }
}

impl<T> Default for WorldGenerator<T, PerThreadRng> {
    fn default() -> Self {
        WorldGenerator {
            rng: PerThreadRng,
            _phantom_storage_type: PhantomData,
        }
    }
}

impl<'a, T> SceneLoader<'a> for WorldGenerator<T>
where T: Storage + Default + Send + Sync + 'static 
{
    type SystemData = (
        Entities<'a>,
        Read<'a, LazyUpdate>,
        Write<'a, T>,
        Write<'a, ActiveWorld>,
    );

    fn load(&mut self, (entities, lazy, mut storage, mut status,): Self::SystemData) {
        let parent = self.pick_scenario(&mut *storage);
        let new_world = match parent {
            Some(ref parent) => self.generate_child_world(&parent.world),
            None => self.generate_new_world(),
        };

        for planet in new_world.planets.iter() {
            let radius = planet.radius();
            let color = self.generate_random_color();
            lazy.create_entity(&entities)
                .with(InScene)
                // Drawing
                .with(DrawColor {
                    fill_color: color,
                    outline_color: color,
                    outline_thickness: 0.,
                })
                .with(DrawShape {
                    shape_type: ShapeType::Circle {
                        radius,
                        point_count: 16,
                    },
                    origin: Vector2f::new(radius, radius),
                })
                // Physics
                .with(Position::new(planet.position, Rotation::from_angle(0.)))
                .with(Velocity {
                    linear: planet.velocity,
                    angular: Rotation::from_angle(0.),
                })
                .with(Mass {
                    linear: planet.mass,
                    angular: 1.,
                })
                .with(ForceAccumulator::default())
                .with(GravitySource)
                .with(GravityTarget)
                .with(CircleCollider::new_in_layer(radius, collision::planet()))
                .build();
        }

        status.world = new_world;
        status.parent = parent;
        status.cumulative_score = 0.;
        status.ticks_completed = 0;
    }
}

impl<T: Storage, R: Rng> WorldGenerator<T, R> {
    /// Picks a scenario to mutate or None if a new scenario should be generated.
    fn pick_scenario(&mut self, storage: &mut T) -> Option<Scenario> {
        let num_scenarios = match storage.num_scenarios() {
            Ok(ns) if ns < 0 => {
                println!("Unexpected negative number of scenarios: {}", ns);
                return None;
            },
            Ok(ns) if ns == 0 => {
                println!("No existing scenarios to mutate, generating new one by default");
                return None;
            }
            Ok(ns) => ns,
            Err(err) => {
                println!("Error getting number of scenarios: {}", err);
                return None;
            },
        };
        let picked_scenario = self.select_index(num_scenarios);
        match storage.get_nth_scenario_by_score(picked_scenario) {
            Ok(Some(scenario)) => {
                println!(
                    "Mutating Scenario {} (parent: {:?}, family: {}, generation: {})",
                    scenario.id,
                    scenario.parent,
                    scenario.family,
                    scenario.generation,
                );
                Some(scenario)
            },
            Ok(None) => {
                println!("Generating new Scenario");
                None
            },
            Err(err) => {
                println!(
                    "Generating new Scenario because of error fetching scenario {}: {}",
                    picked_scenario,
                    err,
                );
                None
            }
        }
    }
    
    /// Selects a random index from the number of scenarios. The selected index may be out of range.
    /// Uses an exponential distribution where the probability of choosing an out of range index (and
    /// thus starting a new scenario) is 5%.
    fn select_index(&mut self, num_items: i64) -> i64 {
        assert!(num_items > 0);
        // The CDF of the exponential distribution is f(x) = 1-e^(-lx). In order to have
        // P probability of getting a value in-range, we want to choose l such that 
        // f(num-scenarios) = P. Therefore we solve for l: 
        // l = -ln(1 - P) / num-scenarios
        // We precompute -ln(1 - P) here as the negative log of the probability of generating a new
        // scenario:
        // -ln(1 - .95) -> -(0.05f64.ln()))
        const NEW_SCENARIO_NEGATIVE_LOG_PROBABILITY: f64 = 2.995732273553991; 
        let lambda = NEW_SCENARIO_NEGATIVE_LOG_PROBABILITY / (num_items as f64 + 1.);
        let dist = Exp::new(lambda);
        dist.sample(&mut self.rng) as i64
    }
}

impl<T, R: Rng> WorldGenerator<T, R> {
    /// Randomly generate a new world.
    fn generate_new_world(&mut self) -> World {
        const MAX_PLANETS: usize = 1000;
        const MIN_PLANETS: usize = 1;
        // -ln(1 - .99999) / 1000 = 99.999% chance of choosing fewer than 1000 planets.
        const NUM_PLANETS_LAMBDA: f64 = 0.01151292546497023;
        let num_planets_dist = Exp::new(NUM_PLANETS_LAMBDA);
        let num_planets = num_planets_dist.sample(&mut self.rng) as usize;
        let num_planets = MAX_PLANETS.min(num_planets);
        let num_planets = MIN_PLANETS.max(num_planets);

        let mut planets = Vec::with_capacity(num_planets);

        for _ in 0..num_planets {
            planets.push(self.generate_new_planet());
        }

        let mut world = World { planets };
        world.merge_overlapping_planets();
        world
    }
    
    /// Mutate the given parent world to generate a new random world.
    fn generate_child_world(&mut self, parent: &World) -> World {
        const MAX_PLANETS_TO_ADD: usize = 20;
        // -ln(1 - .999) / 10 = 99.9% chance of adding fewer than 10 planets.
        const ADD_PLANETS_LAMBDA: f64 = 0.6907755278982136;
        let add_planets_dist = Exp::new(ADD_PLANETS_LAMBDA);
        let num_planets_to_add = add_planets_dist.sample(&mut self.rng) as usize;
        let num_planets_to_add = MAX_PLANETS_TO_ADD.min(num_planets_to_add);

        const MAX_PLANETS_TO_REMOVE: usize = 20;
        // -ln(1 - .999) / 10 = 99.9% chance of removing fewer than 10 planets.
        const REMOVE_PLANETS_LAMBDA: f64 = 0.6907755278982136;
        let remove_planets_dist = Exp::new(REMOVE_PLANETS_LAMBDA);
        let num_planets_to_remove = remove_planets_dist.sample(&mut self.rng) as usize;
        let num_planets_to_remove = MAX_PLANETS_TO_REMOVE.min(num_planets_to_remove);
        let num_planets_to_remove = parent.planets.len().min(num_planets_to_remove);

        // Change about 10% of planets.
        const FRACTION_OF_PLANETS_TO_CHANGE: f64 = 0.10;
        let change_planet_dist = Bernoulli::new(FRACTION_OF_PLANETS_TO_CHANGE);

        // Order of changes is remove, modify, add. This is so we don't remove or modify newly
        // added planets and don't modify planets that are about to be removed.
        
        let mut world = parent.clone();

        // Remove:
        for _ in 0..num_planets_to_remove {
            // panics if start >= end, but this loop doesn't run if planets.len() == 0, so this is
            // safe.
            let selected = Uniform::new(0, world.planets.len()).sample(&mut self.rng);
            world.planets.remove(selected);
        }

        // Modify
        for planet in world.planets.iter_mut() {
            if change_planet_dist.sample(&mut self.rng) {
                self.mutate_planet(planet);
            }
        }

        for _ in 0..num_planets_to_add {
            world.planets.push(self.generate_new_planet());
        }

        world.merge_overlapping_planets();

        world
    }

    /// Generates a new randomly sized planet at a random location with random velocity.
    fn generate_new_planet(&mut self) -> Planet {
        const MIN_X: f32 = -statustracker::SCORED_SCREEN_WIDTH;
        const MAX_X: f32 = statustracker::SCORED_SCREEN_WIDTH;
        const MIN_Y: f32 = -statustracker::SCORED_SCREEN_HEIGHT;
        const MAX_Y: f32 = statustracker::SCORED_SCREEN_HEIGHT;
        let horizontal_dist = Uniform::new_inclusive(MIN_X, MAX_X);
        let vertical_dist = Uniform::new_inclusive(MIN_Y, MAX_Y);

        let position = Vector::new(
            horizontal_dist.sample(&mut self.rng),
            vertical_dist.sample(&mut self.rng),
        );

        const AVERAGE_HORIZONTAL_VELOCITY: f64 = 0.;
        const HORIZONTAL_VELOCITY_STDDEV: f64 = 20.;
        const AVERAGE_VERTICAL_VELOCITY: f64 = 0.;
        const VERTICAL_VELOCITY_STDDEV: f64 = 20.;
        let horizontal_velocity_dist = Normal::new(
            AVERAGE_HORIZONTAL_VELOCITY,
            HORIZONTAL_VELOCITY_STDDEV,
        );
        let vertical_velocity_dist = Normal::new(
            AVERAGE_VERTICAL_VELOCITY,
            VERTICAL_VELOCITY_STDDEV,
        );

        let velocity = Vector::new(
            horizontal_velocity_dist.sample(&mut self.rng) as f32,
            vertical_velocity_dist.sample(&mut self.rng) as f32,
        );

        const MIN_MASS: f32 = 1.;
        const MEAN_MASS: f64 = 500.;
        const MASS_STDDEV: f64 = 400.;
        let mass_dist = Normal::new(MEAN_MASS, MASS_STDDEV);
        let mass = MIN_MASS.max(mass_dist.sample(&mut self.rng) as f32);

        Planet {
            position,
            velocity,
            mass,
        }
    }

    /// Mutates a planet by making small changes to the mass, position, and velocity.
    fn mutate_planet(&mut self, planet: &mut Planet) {
        const H_POS_CHANGE_MEAN: f64 = 0.;
        const H_POS_CHANGE_STDEV: f64 = 10.;
        let h_pos_change_dist = Normal::new(H_POS_CHANGE_MEAN, H_POS_CHANGE_STDEV);
        const V_POS_CHANGE_MEAN: f64 = 0.;
        const V_POS_CHANGE_STDEV: f64 = 10.;
        let v_pos_change_dist = Normal::new(V_POS_CHANGE_MEAN, V_POS_CHANGE_STDEV);
        
        const H_VEL_CHANGE_MEAN: f64 = 0.;
        const H_VEL_CHANGE_STDEV: f64 = 10.;
        let h_vel_change_dist = Normal::new(H_VEL_CHANGE_MEAN, H_VEL_CHANGE_STDEV);
        const V_VEL_CHANGE_MEAN: f64 = 0.;
        const V_VEL_CHANGE_STDEV: f64 = 10.;
        let v_vel_change_dist = Normal::new(V_VEL_CHANGE_MEAN, V_VEL_CHANGE_STDEV);

        const MASS_CHANGE_MEAN: f64 = 0.;
        const MASS_CHANGE_STDEV: f64 = 100.;
        const MIN_MASS: f32 = 1.;
        let mass_change_dist = Normal::new(MASS_CHANGE_MEAN, MASS_CHANGE_STDEV);

        planet.position.x += h_pos_change_dist.sample(&mut self.rng) as f32;
        planet.position.y += v_pos_change_dist.sample(&mut self.rng) as f32;
        planet.velocity.x += h_vel_change_dist.sample(&mut self.rng) as f32;
        planet.velocity.y += v_vel_change_dist.sample(&mut self.rng) as f32;
        planet.mass += mass_change_dist.sample(&mut self.rng) as f32;
        planet.mass = MIN_MASS.max(planet.mass);
    }

    /// Generates a random color, usually fairly bright.
    fn generate_random_color(&mut self) -> Color {
        let hue_dist = Uniform::new(0., 360.);
        let sat_dist = Uniform::new_inclusive(0.75, 1.);
        let value_dist = Uniform::new_inclusive(0.75, 1.);

        let h = hue_dist.sample(&mut self.rng);
        let s = sat_dist.sample(&mut self.rng);
        let v = value_dist.sample(&mut self.rng);
        hsv_to_rgb(h, s, v)
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    assert!(0. <= h && h < 360.);
    assert!(0. <= s && s <= 1.);
    assert!(0. <= v && v <= 1.);
    let (r, g, b) = if s == 0. {
        (v, v, v)
    } else {
        let hh = h / 60.;
        let i = hh.trunc() as i32;
        let ff = hh.fract();
        let p = v * (1.0 - s);
        let q = v * (1.0 - (s * ff));
        let t = v * (1.0 - (s * (1.0 - ff)));

        match i {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            5 => (v, p, q),
            _ => panic!("unexpected sector index: {}" , i),
        }
    };
    Color::rgb(
        (255. * r).round() as u8,
        (255. * g).round() as u8,
        (255. * b).round() as u8,
    )
}
