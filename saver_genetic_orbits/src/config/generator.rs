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

//! Contains configuration structs for the world generator.

use physics::components::Vector;

use crate::config::{
    util::{
        Distribution,
        ExponentialDistribution,
        NormalDistribution,
        Range,
        UniformDistribution,
        Vector as SerVec,
    },
    Validation,
    fix_invalid_helper,
};

/// Tuning parameters for the world generator/mutator.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GeneratorConfig {
    /// The probability of generating a new scenario. Parent scenarios are chosen with an
    /// exponential distribution over all current scenarios. The lambda for the exponential
    /// distribution is chosen so that there is `create_new_scenario_probability` of getting an
    /// index outside of the existing scenario range, which triggers generating a new scenario.
    pub create_new_scenario_probability: f64,

    /// The parameters affecting world mutation.
    pub mutation_parameters: MutationParameters,

    /// The parameters affecting new world generation.
    pub new_world_parameters: NewWorldParameters,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        GeneratorConfig {
            create_new_scenario_probability: 0.05,
            mutation_parameters: Default::default(),
            new_world_parameters: Default::default(),
        }
    }
}

impl Validation for GeneratorConfig {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "create_new_scenario_probability", "must be in range (0, 1) (exclusive)",
            &mut self.create_new_scenario_probability,
            |&v| v > 0. && v < 1., 
            || Self::default().create_new_scenario_probability,
        );
        // join precomputes the total length, which guarantees exactly 1 allocation, whereas
        // path.to_string() + ".xyz" creates a string with exactly path.len() capactiy then
        // reallocates to extend it.
        self.mutation_parameters.fix_invalid(&[path, "mutation_parameters"].join("."));
        self.new_world_parameters.fix_invalid(&[path, "new_world_parameters"].join("."));
    }
}

/// Parameters that control initial world generation.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MutationParameters {
    /// The min and max number of planets to add. Used as a clamp on the add_planets_distribution.
    /// Defaults to [0, 20]. Max is inclusive.
    pub add_planets_limits: Range<usize>,

    /// Distribution over the number of new planets to add. If using a uniform distribution, the
    /// range is inclusive. Exponential distribution rounds down, normal distribution rounds to
    /// nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.9% chance
    /// of having fewer than 10 new planets.
    pub add_planets_dist: Distribution,

    /// The parameters affecting new planets that get added in this mutation.
    pub new_planet_parameters: NewPlanetParameters,

    /// The min and max number of planets to remove. Used as a clamp on the
    /// remove_planets_distribution.  Defaults to [0, 20]. Max is inclusive.
    pub remove_planets_limits: Range<usize>,

    /// Distribution over the number of new planets to remove. If using a uniform distribution, the
    /// range is inclusive. Exponential distribution rounds down, normal distribution rounds to
    /// nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.9% chance
    /// of removing fewer than 10 planets.
    pub remove_planets_dist: Distribution,

    /// Percentage of planets to change, on average.
    pub fraction_of_planets_to_change: f64,

    /// Parameters for how to mutate individual planets.
    pub planet_mutation_parameters: PlanetMutationParameters,
}

impl Default for MutationParameters {
    fn default() -> Self {
        const DEFAULT_ADD_REMOVE_PLANETS_LIMITS: Range<usize> = Range {
            min: 0,
            max: 20,
        };
        // -ln(1 - .999) / 10 = 99.9% chance of adding or removing fewer than 10 planets.
        const DEFAULT_ADD_REMOVE_PLANETS_DIST: Distribution =
            Distribution::Exponential(ExponentialDistribution(0.6907755278982136));
        MutationParameters {
            add_planets_limits: DEFAULT_ADD_REMOVE_PLANETS_LIMITS,
            add_planets_dist: DEFAULT_ADD_REMOVE_PLANETS_DIST,
            new_planet_parameters: Default::default(),
            remove_planets_limits: DEFAULT_ADD_REMOVE_PLANETS_LIMITS,
            remove_planets_dist: DEFAULT_ADD_REMOVE_PLANETS_DIST,
            fraction_of_planets_to_change: 0.10,
            planet_mutation_parameters: Default::default(),
        }
    }
}
    
impl Validation for MutationParameters {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "add_planets_limits", "must have min >= 0 and max >= min",
            &mut self.add_planets_limits,
            |v| v.max >= v.min,
            || Self::default().add_planets_limits,
        );
        self.add_planets_dist.fix_invalid_helper_iu(
            path, "add_planets_dist", || Self::default().add_planets_dist);
        self.new_planet_parameters.fix_invalid(&[path, "new_planet_parameters"].join("."));
        fix_invalid_helper(
            path, "remove_planets_limits", "must have min >= 0 and max >= min",
            &mut self.remove_planets_limits,
            |v| v.max >= v.min,
            || Self::default().remove_planets_limits,
        );
        self.remove_planets_dist.fix_invalid_helper_iu(
            path, "remove_planets_dist", || Self::default().remove_planets_dist);
        fix_invalid_helper(
            path, "fraction_of_planets_to_change", "must be in range [0, 1] (inclusive)",
            &mut self.fraction_of_planets_to_change, |&v| v >= 0. && v <= 1.,
            || Self::default().fraction_of_planets_to_change,
        );
        self.planet_mutation_parameters
            .fix_invalid(&[path, "planet_mutation_parameters"].join("."));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct NewWorldParameters {
    /// Inclusive range over the number of planets that should be generated. Used to cap
    /// distributions with long tails. Defaults to [1, 1000].
    pub num_planets_range: Range<usize>,
    /// Distribution used for selecting the number of planets to distribute over. If using a
    /// uniform distribution, the range is inclusive. Exponential distribution rounds down, normal
    /// distribution rounds to nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.999%
    /// chance of picking fewer than 1000 planets.
    pub num_planets_dist: Distribution,
    /// Parameters for how new planets are generated.
    pub planet_parameters: NewPlanetParameters,
}

impl Default for NewWorldParameters {
    fn default() -> Self {
        NewWorldParameters {
            num_planets_range: Range { min: 1, max: 1000 },
            num_planets_dist:
                // -ln(1 - .99999) / 1000 = 99.999% chance of choosing fewer than 1000 planets.
                Distribution::Exponential(ExponentialDistribution(0.01151292546497023)),
            planet_parameters: Default::default(),
        }
    }
}

impl Validation for NewWorldParameters {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "num_planets_range", "must have min >= 0 and max >= min",
            &mut self.num_planets_range,
            |v| v.max >= v.min,
            || Self::default().num_planets_range,
        );
        self.num_planets_dist.fix_invalid_helper_iu(
            path, "num_planets_dist", || Self::default().num_planets_dist);
        self.planet_parameters.fix_invalid(&[path, "planet_paramters"].join("."));
    }
}

/// Parameters to control how new planets are generated.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct NewPlanetParameters {
    /// The distribution the generated planet's start position. Defaults to [-2000, 2000] in
    /// both x and y to match the default scored area. Planets are spawned in a uniform
    /// distribution over this area. Both endpoints are inclusive.
    pub start_position: UniformDistribution<Vector>,
    // TODO(zstewar1): support scale_width_by_aspect.
    /// Controls the distribution of starting velocities for planets. Defaults to mean: 0,
    /// stddev:
    /// 20 in both x and y.
    pub start_velocity: SerVec<NormalDistribution>,
    /// A minimum limit on the starting mass of planets. Should be positve (i.e. greater than
    /// zero). defaults to 1.
    pub min_start_mass: f32,
    /// Controls the distribution of starting masses for planets. Defaults to mean: 500.
    /// stddev: 400.
    pub start_mass: NormalDistribution,
}

impl Default for NewPlanetParameters {
    fn default() -> Self {
        NewPlanetParameters {
            start_position: UniformDistribution {
                min: Vector::new(-2000., -2000.),
                max: Vector::new(2000., 2000.),
            },
            start_velocity: SerVec {
                x: NormalDistribution {
                    mean: 0.,
                    standard_deviation: 20.
                },
                y: NormalDistribution {
                    mean: 0.,
                    standard_deviation: 20.
                },
            },
            min_start_mass: 1.,
            start_mass: NormalDistribution {
                mean: 500.,
                standard_deviation: 400.,
            },
        }
    }
}

impl Validation for NewPlanetParameters {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "start_position", "must have min.x <= max.x and min.y <= max.y",
            &mut self.start_position,
            |v| v.min.x <= v.max.x && v.min.y <= v.max.y,
            || Self::default().start_position,
        );
        fix_invalid_helper(
            path, "start_velocity", "must have standard_deviation >= 0 in both x and y",
            &mut self.start_velocity,
            |v| v.x.standard_deviation >= 0. && v.y.standard_deviation >= 0.,
            || Self::default().start_velocity,
        );
        fix_invalid_helper(
            path, "min_start_mass", "must be > 0",
            &mut self.min_start_mass,
            |&v| v > 0.,
            || Self::default().min_start_mass,
        );
        fix_invalid_helper(
            path, "start_mass", "must have standard_deviation >= 0",
            &mut self.start_mass,
            |v| v.standard_deviation >= 0.,
            || Self::default().start_mass,
        );
    }
}

/// Parameters to control how planets are mutated.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct PlanetMutationParameters {
    /// Distribution for how much to change position when modifying the planet. Defaults to a mean
    /// of 0 and a standard deviation of 10 in both x and y.
    pub position_change: SerVec<NormalDistribution>,

    /// Distribution for how much to change velocity when modifying the planet. Defaults to a mean
    /// of 0 and a standard deviation of 10 in both x and y.
    pub velocity_change: SerVec<NormalDistribution>,

    /// Distribution for how much to change mass when modifying the planet. Defaults to a normal
    /// distribution with a mean of 0 and a standard deviation of 100.
    pub mass_change: Distribution,

    /// Min mass that the planet must have, used to clamp the results of the mass change must be
    /// positive. Default is 1.
    pub min_mass: f32,
}

impl Default for PlanetMutationParameters {
    fn default() -> Self {
        const DEFAULT_VEC_CHANGE: SerVec<NormalDistribution> = SerVec {
            x: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
            y: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
        };
        PlanetMutationParameters {
            position_change: DEFAULT_VEC_CHANGE,
            velocity_change: DEFAULT_VEC_CHANGE,
            mass_change: Distribution::Normal(NormalDistribution {
                mean: 0.,
                standard_deviation: 100.,
            }),
            min_mass: 1.,
        }
    }
}

impl Validation for PlanetMutationParameters {
    fn fix_invalid(&mut self, path: &str) {
        fix_invalid_helper(
            path, "position_change", "must have standard_deviation >= 0 in both x and y",
            &mut self.position_change,
            |v| v.x.standard_deviation >= 0. && v.y.standard_deviation >= 0.,
            || Self::default().position_change,
        );
        fix_invalid_helper(
            path, "velocity_change", "must have standard_deviation >= 0 in both x and y",
            &mut self.velocity_change,
            |v| v.x.standard_deviation >= 0. && v.y.standard_deviation >= 0.,
            || Self::default().velocity_change,
        );
        fix_invalid_helper(
            path, "mass_change", "must not use the exponential distribution",
            &mut self.mass_change,
            |v| match v {
                &Distribution::Exponential(_) => false,
                _ => true,
            },
            || Self::default().mass_change,
        );
        self.mass_change.fix_invalid_helper_iu(path, "mass_change", || Self::default().mass_change);
        fix_invalid_helper(
            path, "min_mass", "must be > 0",
            &mut self.min_mass,
            |&v| v > 0.,
            || Self::default().min_mass,
        );
    }
}

