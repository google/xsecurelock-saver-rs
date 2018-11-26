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

use xsecurelock_saver::engine::components::physics::Vector;
use config::util::{
    Distribution,
    ExponentialDistribution,
    NormalDistribution,
    Range,
    UniformDistribution,
    Vector as SerVec,
};

/// Tuning parameters for the world generator/mutator.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneratorConfig {
    /// The probability of generating a new scenario. Parent scenarios are chosen with an
    /// exponential distribution over all current scenarios. The lambda for the exponential
    /// distribution is chosen so that there is `create_new_scenario_probability` of getting an
    /// index outside of the existing scenario range, which triggers generating a new scenario.
    #[serde(default = "GeneratorConfig::default_create_new_scenario_probability")]
    pub create_new_scenario_probability: f64,

    /// The parameters affecting world mutation.
    #[serde(default)]
    pub mutation_parameters: MutationParameters,

    /// The parameters affecting new world generation.
    #[serde(default)]
    pub new_world_parameters: NewWorldParameters,
}

impl GeneratorConfig {
    fn default_create_new_scenario_probability() -> f64 { 0.05 }
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        GeneratorConfig {
            create_new_scenario_probability: Self::default_create_new_scenario_probability(),
            mutation_parameters: Default::default(),
            new_world_parameters: Default::default(),
        }
    }
}

/// Parameters that control initial world generation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MutationParameters {
    /// The min and max number of planets to add. Used as a clamp on the add_planets_distribution.
    /// Defaults to [0, 20]. Max is inclusive.
    #[serde(default = "MutationParameters::default_add_planets_limits")]
    pub add_planets_limits: Range<usize>,

    /// Distribution over the number of new planets to add. If using a uniform distribution, the
    /// range is inclusive. Exponential distribution rounds down, normal distribution rounds to
    /// nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.9% chance
    /// of having fewer than 10 new planets.
    #[serde(default = "MutationParameters::default_add_planets_dist")]
    pub add_planets_dist: Distribution,

    /// The parameters affecting new planets that get added in this mutation.
    #[serde(default)]
    pub new_planet_parameters: NewPlanetParameters,

    /// The min and max number of planets to remove. Used as a clamp on the
    /// remove_planets_distribution.  Defaults to [0, 20]. Max is inclusive.
    #[serde(default = "MutationParameters::default_remove_planets_limits")]
    pub remove_planets_limits: Range<usize>,

    /// Distribution over the number of new planets to remove. If using a uniform distribution, the
    /// range is inclusive. Exponential distribution rounds down, normal distribution rounds to
    /// nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.9% chance
    /// of removing fewer than 10 planets.
    #[serde(default = "MutationParameters::default_remove_planets_dist")]
    pub remove_planets_dist: Distribution,

    /// Percentage of planets to change, on average.
    #[serde(default = "MutationParameters::default_fraction_of_planets_to_change")]
    pub fraction_of_planets_to_change: f64,

    /// Parameters for how to mutate individual planets.
    #[serde(default)]
    pub planet_mutation_parameters: PlanetMutationParameters,
}

impl MutationParameters {
    fn default_add_planets_limits() -> Range<usize> {
        Range {
            min: 0,
            max: 20,
        }
    }

    fn default_add_planets_dist() -> Distribution {
        // -ln(1 - .999) / 10 = 99.9% chance of adding fewer than 10 planets.
        Distribution::Exponential(ExponentialDistribution(0.6907755278982136))
    }

    fn default_remove_planets_limits() -> Range<usize> {
        Range {
            min: 0,
            max: 20,
        }
    }

    fn default_remove_planets_dist() -> Distribution {
        // -ln(1 - .999) / 10 = 99.9% chance of removing fewer than 10 planets.
        Distribution::Exponential(ExponentialDistribution(0.6907755278982136))
    }

    fn default_fraction_of_planets_to_change() -> f64 { 0.10 }
}

impl Default for MutationParameters {
    fn default() -> Self {
        MutationParameters {
            add_planets_limits: Self::default_add_planets_limits(),
            add_planets_dist: Self::default_add_planets_dist(),
            new_planet_parameters: Default::default(),
            remove_planets_limits: Self::default_remove_planets_limits(),
            remove_planets_dist: Self::default_remove_planets_dist(),
            fraction_of_planets_to_change: Self::default_fraction_of_planets_to_change(),
            planet_mutation_parameters: Default::default(),
        }
    }
}
    
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewWorldParameters {
    /// Inclusive range over the number of planets that should be generated. Used to cap
    /// distributions with long tails. Defaults to [1, 1000].
    #[serde(default = "NewWorldParameters::default_num_planets_range")]
    pub num_planets_range: Range<usize>,
    /// Distribution used for selecting the number of planets to distribute over. If using a
    /// uniform distribution, the range is inclusive. Exponential distribution rounds down, normal
    /// distribution rounds to nearest.
    /// The default value is an exponential distribution with lambda chosen to have a 99.999%
    /// chance of picking fewer than 1000 planets.
    #[serde(default = "NewWorldParameters::default_num_planets_dist")]
    pub num_planets_dist: Distribution,
    /// Parameters for how new planets are generated.
    #[serde(default)]
    pub planet_parameters: NewPlanetParameters,
}

impl NewWorldParameters {
    fn default_num_planets_range() -> Range<usize> { Range { min: 1, max: 1000 } }
    fn default_num_planets_dist() -> Distribution {
        // -ln(1 - .99999) / 1000 = 99.999% chance of choosing fewer than 1000 planets.
        Distribution::Exponential(ExponentialDistribution(0.01151292546497023))
    }
}

impl Default for NewWorldParameters {
    fn default() -> Self {
        NewWorldParameters {
            num_planets_range: Self::default_num_planets_range(),
            num_planets_dist: Self::default_num_planets_dist(),
            planet_parameters: Default::default(),
        }
    }
}



/// Parameters to control how new planets are generated.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewPlanetParameters {
    /// The distribution the generated planet's start position. Defaults to [-2000, 2000] in
    /// both x and y to match the default scored area. Planets are spawned in a uniform
    /// distribution over this area. Both endpoints are inclusive.
    #[serde(default = "NewPlanetParameters::default_start_position")]
    pub start_position: UniformDistribution<Vector>,
    // TODO(zstewar1): support scale_width_by_aspect.
    /// Controls the distribution of starting velocities for planets. Defaults to mean: 0,
    /// stddev:
    /// 20 in both x and y.
    #[serde(default = "NewPlanetParameters::default_start_velocity")]
    pub start_velocity: SerVec<NormalDistribution>,
    /// A minimum limit on the starting mass of planets. Should be positve (i.e. greater than
    /// zero). defaults to 1.
    #[serde(default = "NewPlanetParameters::default_min_start_mass")]
    pub min_start_mass: f32,
    /// Controls the distribution of starting masses for planets. Defaults to mean: 500.
    /// stddev: 400.
    #[serde(default = "NewPlanetParameters::default_start_mass")]
    pub start_mass: NormalDistribution,
}

impl NewPlanetParameters {
    fn default_start_position() -> UniformDistribution<Vector> {
        UniformDistribution {
            min: Vector::new(-2000., 2000.),
            max: Vector::new(-2000., 2000.),
        }
    }
    
    fn default_start_velocity() -> SerVec<NormalDistribution> {
        SerVec {
            x: NormalDistribution {
                mean: 0.,
                standard_deviation: 20.
            },
            y: NormalDistribution {
                mean: 0.,
                standard_deviation: 20.
            },
        }
    }
    
    fn default_min_start_mass() -> f32 { 1. }
    
    fn default_start_mass() -> NormalDistribution {
        NormalDistribution {
            mean: 500.,
            standard_deviation: 400.,
        }
    }
}

impl Default for NewPlanetParameters {
    fn default() -> Self {
        NewPlanetParameters {
            start_position: Self::default_start_position(),
            start_velocity: Self::default_start_velocity(),
            min_start_mass: Self::default_min_start_mass(),
            start_mass: Self::default_start_mass(),
        }
    }
}

/// Parameters to control how planets are mutated.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanetMutationParameters {
    /// Distribution for how much to change position when modifying the planet. Defaults to a mean
    /// of 0 and a standard deviation of 10 in both x and y.
    #[serde(default = "PlanetMutationParameters::default_position_change")]
    pub position_change: SerVec<NormalDistribution>,

    /// Distribution for how much to change velocity when modifying the planet. Defaults to a mean
    /// of 0 and a standard deviation of 10 in both x and y.
    #[serde(default = "PlanetMutationParameters::default_velocity_change")]
    pub velocity_change: SerVec<NormalDistribution>,

    /// Distribution for how much to change mass when modifying the planet. Defaults to a normal
    /// distribution with a mean of 0 and a standard deviation of 100.
    #[serde(default = "PlanetMutationParameters::default_mass_change")]
    pub mass_change: Distribution,

    /// Min mass that the planet must have, used to clamp the results of the mass change must be
    /// positive. Default is 1.
    #[serde(default = "PlanetMutationParameters::default_min_mass")]
    pub min_mass: f32,
}

impl PlanetMutationParameters {
    fn default_position_change() -> SerVec<NormalDistribution> {
        SerVec {
            x: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
            y: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
        }
    }

    fn default_velocity_change() -> SerVec<NormalDistribution> {
        SerVec {
            x: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
            y: NormalDistribution {
                mean: 0.,
                standard_deviation: 10.,
            },
        }
    }

    fn default_mass_change() -> Distribution {
        Distribution::Normal(NormalDistribution {
            mean: 0.,
            standard_deviation: 100.,
        })
    }

    fn default_min_mass() -> f32 { 1. }
}

impl Default for PlanetMutationParameters {
    fn default() -> Self {
        PlanetMutationParameters {
            position_change: Self::default_position_change(),
            velocity_change: Self::default_velocity_change(),
            mass_change: Self::default_mass_change(),
            min_mass: Self::default_min_mass(),
        }
    }
}
