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
use statustracker::ActiveWorld;
use storage::Storage;
use config::{
    generator::{
        GeneratorConfig,
        MutationParameters,
        NewPlanetParameters,
        NewWorldParameters,
        PlanetMutationParameters,
    },
    util::{
        Distribution as ConfDist,
        ExponentialDistribution,
        NormalDistribution,
        UniformDistribution,
    },
};
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
        Read<'a, GeneratorConfig>,
        Read<'a, LazyUpdate>,
        Write<'a, T>,
        Write<'a, ActiveWorld>,
    );

    fn load(&mut self, (entities, config, lazy, mut storage, mut status,): Self::SystemData) {
        let parent = self.pick_scenario(&mut *storage, &*config);
        let new_world = match parent {
            Some(ref parent) => self.generate_child_world(
                &parent.world, &config.mutation_parameters),
            None => self.generate_new_world(&config.new_world_parameters),
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
                        point_count: radius_to_point_count(radius),
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
    fn pick_scenario(&mut self, storage: &mut T, config: &GeneratorConfig) -> Option<Scenario> {
        let num_scenarios = match storage.num_scenarios() {
            Ok(ns) if ns < 0 => {
                error!("Unexpected negative number of scenarios: {}", ns);
                return None;
            },
            Ok(ns) if ns == 0 => {
                info!("No existing scenarios to mutate, generating new one by default");
                return None;
            }
            Ok(ns) => ns,
            Err(err) => {
                error!("Error getting number of scenarios: {}", err);
                return None;
            },
        };
        let picked_scenario = self.select_index(num_scenarios, config);
        match storage.get_nth_scenario_by_score(picked_scenario) {
            Ok(Some(scenario)) => {
                info!(
                    "Mutating Scenario {} (parent: {:?}, family: {}, generation: {})",
                    scenario.id,
                    scenario.parent,
                    scenario.family,
                    scenario.generation,
                );
                Some(scenario)
            },
            Ok(None) => {
                info!("Generating new Scenario");
                None
            },
            Err(err) => {
                error!(
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
    fn select_index(&mut self, num_items: i64, config: &GeneratorConfig) -> i64 {
        assert!(num_items > 0);
        // The CDF of the exponential distribution is f(x) = 1-e^(-lx). In order to have
        // P probability of getting a value in-range, we want to choose l such that 
        // f(num-scenarios) = P. Therefore we solve for l: 
        // l = -ln(1 - P) / num-scenarios
        let lambda = -(config.create_new_scenario_probability.ln()) / (num_items as f64 + 1.);
        let dist = Exp::new(lambda);
        dist.sample(&mut self.rng) as i64
    }
}

impl<T, R: Rng> WorldGenerator<T, R> {
    /// Randomly generate a new world.
    fn generate_new_world(&mut self, params: &NewWorldParameters) -> World {
        let num_planets = match params.num_planets_dist {
            ConfDist::Exponential(ExponentialDistribution(lambda)) => 
                Exp::new(lambda).sample(&mut self.rng) as usize,
            ConfDist::Normal(NormalDistribution{mean, standard_deviation}) => 
                Normal::new(mean, standard_deviation).sample(&mut self.rng).round() as usize,
            ConfDist::Uniform(UniformDistribution{min, max}) => 
                Uniform::new_inclusive(min as usize, max as usize).sample(&mut self.rng),
        };
        let num_planets = params.num_planets_range.clamp_inclusive(num_planets);
        info!("Generating {} planets", num_planets);

        let mut planets = Vec::with_capacity(num_planets);
        for _ in 0..num_planets {
            planets.push(self.generate_new_planet(&params.planet_parameters));
        }

        let mut world = World { planets };
        world.merge_overlapping_planets();
        info!("After overlap cleanup, world had {} planets", world.planets.len());
        world
    }
    
    /// Mutate the given parent world to generate a new random world.
    fn generate_child_world(&mut self, parent: &World, params: &MutationParameters) -> World {
        let num_planets_to_add = match params.add_planets_dist {
            ConfDist::Exponential(ExponentialDistribution(lambda)) => 
                Exp::new(lambda).sample(&mut self.rng) as usize,
            ConfDist::Normal(NormalDistribution{mean, standard_deviation}) => 
                Normal::new(mean, standard_deviation).sample(&mut self.rng).round() as usize,
            ConfDist::Uniform(UniformDistribution{min, max}) => 
                Uniform::new_inclusive(min as usize, max as usize).sample(&mut self.rng),
        };
        let num_planets_to_add = params.add_planets_limits
            .clamp_inclusive(num_planets_to_add);

        let num_planets_to_remove = match params.remove_planets_dist {
            ConfDist::Exponential(ExponentialDistribution(lambda)) => 
                Exp::new(lambda).sample(&mut self.rng) as usize,
            ConfDist::Normal(NormalDistribution{mean, standard_deviation}) => 
                Normal::new(mean, standard_deviation).sample(&mut self.rng).round() as usize,
            ConfDist::Uniform(UniformDistribution{min, max}) => 
                Uniform::new_inclusive(min as usize, max as usize).sample(&mut self.rng),
        };
        let num_planets_to_remove = params.remove_planets_limits
            .clamp_inclusive(num_planets_to_remove);
        let num_planets_to_remove = parent.planets.len().min(num_planets_to_remove);

        let change_planet_dist = Bernoulli::new(params.fraction_of_planets_to_change);

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
        info!("Removed {} planets" , num_planets_to_remove);

        // Modify
        let mut num_modified = 0;
        for planet in world.planets.iter_mut() {
            if change_planet_dist.sample(&mut self.rng) {
                self.mutate_planet(planet, &params.planet_mutation_parameters);
                num_modified += 1;
            }
        }
        info!("Modified {} planets" , num_modified);

        for _ in 0..num_planets_to_add {
            world.planets.push(self.generate_new_planet(&params.new_planet_parameters));
        }
        info!("Added {} planets" , num_planets_to_add);

        world.merge_overlapping_planets();
        info!("After overlap cleanup, world had {} planets", world.planets.len());
        world
    }

    /// Generates a new randomly sized planet at a random location with random velocity.
    fn generate_new_planet(&mut self, params: &NewPlanetParameters) -> Planet {
        let horizontal_dist = Uniform::new_inclusive(
            params.start_position.min.x,
            params.start_position.max.x
        );
        let vertical_dist = Uniform::new_inclusive(
            params.start_position.min.y,
            params.start_position.max.y
        );

        let position = Vector::new(
            horizontal_dist.sample(&mut self.rng),
            vertical_dist.sample(&mut self.rng),
        );

        let horizontal_velocity_dist = Normal::new(
            params.start_velocity.x.mean,
            params.start_velocity.x.standard_deviation,
        );
        let vertical_velocity_dist = Normal::new(
            params.start_velocity.y.mean,
            params.start_velocity.y.standard_deviation,
        );

        let velocity = Vector::new(
            horizontal_velocity_dist.sample(&mut self.rng) as f32,
            vertical_velocity_dist.sample(&mut self.rng) as f32,
        );

        let mass_dist = Normal::new(params.start_mass.mean, params.start_mass.standard_deviation);
        let mass = params.min_start_mass.max(mass_dist.sample(&mut self.rng) as f32);

        Planet {
            position,
            velocity,
            mass,
        }
    }

    /// Mutates a planet by making small changes to the mass, position, and velocity.
    fn mutate_planet(&mut self, planet: &mut Planet, params: &PlanetMutationParameters) {
        let h_pos_change = Normal::new(
            params.position_change.x.mean, params.position_change.x.standard_deviation,
        ).sample(&mut self.rng) as f32;
        let v_pos_change = Normal::new(
            params.position_change.y.mean, params.position_change.y.standard_deviation,
        ).sample(&mut self.rng) as f32;
        
        let h_vel_change = Normal::new(
            params.velocity_change.x.mean, params.velocity_change.x.standard_deviation,
        ).sample(&mut self.rng) as f32;
        let v_vel_change = Normal::new(
            params.velocity_change.y.mean, params.velocity_change.y.standard_deviation,
        ).sample(&mut self.rng) as f32;

        let mass_change = match params.mass_change {
            ConfDist::Exponential(ExponentialDistribution(lambda)) => 
                Exp::new(lambda).sample(&mut self.rng),
            ConfDist::Normal(NormalDistribution{mean, standard_deviation}) => 
                Normal::new(mean, standard_deviation).sample(&mut self.rng),
            ConfDist::Uniform(UniformDistribution{min, max}) => 
                Uniform::new_inclusive(min, max).sample(&mut self.rng),
        } as f32;

        planet.position.x += h_pos_change;
        planet.position.y += v_pos_change;
        planet.velocity.x += h_vel_change;
        planet.velocity.y += v_vel_change;
        planet.mass += mass_change;
        planet.mass = params.min_mass.max(planet.mass);
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

pub fn radius_to_point_count(radius: f32) -> u32 {
    const MIN_SEGMENTS: u32 = 8;
    const SEGMENT_LEN: f32 = 8.;
    let circumfrence = 2. * ::std::f32::consts::PI * radius;
    MIN_SEGMENTS.max((circumfrence / SEGMENT_LEN).ceil() as u32)
}
